
<script lang="ts">
    const SVG_SPEC = 'http://www.w3.org/2000/svg';

    import { type AdjData, type PosData, type MapMeta, type GameMeta, type MapState, type Orders,  type MoveOrder, type Builds, type Unit, type GamePhase, type GamePhaseYear, type RetreatOptions, type MvmtInfo, PHASES, prevPhase, nextPhase, isBuild, unitNatl, isRetreat, nextNonemptyPhase, prevNonemptyPhase } from './defs';

    let mapDiv: HTMLDivElement;

    let adjData: AdjData;
    let posData: PosData;
    let metaData: MapMeta;

    function sleep(ms: number) {
        return new Promise((res) => {
            setTimeout(res, ms);
        });
    }

    type Message = 
        { type: "error", msg: string } |
        { type: "update_players", players: Array<[string, string]> } | 
        { type: "game_info", power: string } | 
        { type: "map_state", year : number, phase: GamePhase, state: MapState } |
        { type: "phase", year : number, phase: GamePhase, adj_time: number, state: MapState } |
        { type: "movement_adj", year : number, phase: GamePhase, orders: Orders, order_status: Record<string, boolean>, retreats: Record<string, RetreatOptions> } |
        { type: "retreat_adj", year : number, phase: GamePhase, orders: Orders } |
        { type: "build_adj", year: number, phase: GamePhase, builds: Builds };

    let playerList: [string, string][] = [];
    let mePower = "";

    let active_phase: GamePhaseYear | null = null;
    let current_phase: GamePhaseYear | null = null;
    let adjTime: number = 0;

    let active_prov: string | null = null;
    let active_prov2: string | null = null;
    let order_mode: "move" | "convoy" | "support" = "move";

    let current_orders: Orders = {};
    let current_builds: Builds = {};

    $: {
        active_phase;
        active_prov = active_prov2 = null;
    }

    $: {
        let all = document.querySelectorAll(".prov-active");
        for (let elem of all) elem.classList.remove("prov-active");

        if (active_prov) {
            document.getElementById(active_prov)?.classList.add("prov-active");
        }
    }
    $: {
        let all = document.querySelectorAll(".prov-active-2");
        for (let elem of all) elem.classList.remove("prov-active-2");

        if (active_prov2) {
            document.getElementById(active_prov2)?.classList.add("prov-active-2");
        }
    }

    function unitLoc(prov: string, unit?: Unit) : string {
        if (active_phase == null) { return prov }

        // can't use units for reasons
        if (!unit) {
            unit = all_states[active_phase] && all_states[active_phase].units[prov];
        }
        if (unit && unit.type == "fleet" && unit.data[1] != "") {
            return prov + "-" + unit.data[1]
        } else {
            return prov;
        }
    }

    function createDisband(x: number, y: number) : SVGElement {
        x = x / posData.width;
        y = y / posData.width;
        let r = 16 / posData.width;

        let g = document.createElementNS(SVG_SPEC, "g");
        g.innerHTML = `
            <line x1=${computeX(x - r)} x2=${computeX(x + r)} y1=${computeX(y - r)} y2=${computeX(y + r)} />
            <line x1=${computeX(x + r)} x2=${computeX(x - r)} y1=${computeX(y - r)} y2=${computeX(y + r)} />
        `;
        g.classList.add("disband");
        svgElem.appendChild(g);
        return g;
    }

    let active_builds: Readonly<Builds> = current_builds;
    $: active_builds = active_phase == current_phase ? current_builds : ((active_phase && all_builds[active_phase]) || {});
    // draw builds
    $: {
        document.querySelectorAll(".build").forEach(n => n.remove());
        document.querySelectorAll(".disband-build").forEach(n => n.remove());

        let ownership = active_phase && all_states[active_phase] &&  all_states[active_phase].ownership;
        if (ownership) for (let prov in active_builds) {
            let build = active_builds[prov];
            let loc = prov + ("coast" in build && build.coast ? "-" + build.coast : "");
            let pos = posData.provinces[loc];

            switch (build.type) {
            case "disband": {
                createDisband(pos.x, pos.y).classList.add("disband-build");
                break;
            }
            case "army": {
                let unit = createUnit("army", ownership[prov], pos.x, pos.y);
                unit.classList.remove("unit");
                unit.classList.add("build");
                break;
            }
            case "fleet": {
                let unit = createUnit("fleet", ownership[prov], pos.x, pos.y);
                unit.classList.remove("unit");
                unit.classList.add("build");
                break;
            }
            }
        }
    }

    let active_orders: Readonly<Orders> = current_orders;
    $: active_orders = active_phase == current_phase ? current_orders : ((active_phase && all_orders[active_phase]) || {});
    // draw orders
    $: {
        document.querySelectorAll(".order").forEach(n => n.remove());

        let order_status = (active_phase && all_mvmt_info[active_phase] && all_mvmt_info[active_phase].order_status) || null;

//        console.log("active orders", active_orders, all_orders);
        for (let prov in active_orders) {
            let order = active_orders[prov];

            switch (order.type) {
            case "move": {
                let src = unitLoc(prov);
                if (active_phase && isRetreat(active_phase) && all_mvmt_info[prevPhase(active_phase)] && prov in all_mvmt_info[prevPhase(active_phase)].retreats) {
                    src = unitLoc(prov, all_mvmt_info[prevPhase(active_phase)].retreats[prov].src);
                }
                let dest = order.dest[0] + "" + (order.dest[1] ? "-" + order.dest[1] : "");

                let x1 = posData.provinces[src].x,
                    y1 = posData.provinces[src].y,
                    x2 = posData.provinces[dest].x,
                    y2 = posData.provinces[dest].y;
                
                if (active_phase && all_states[active_phase].units[prov] && isRetreat(active_phase)) {
                    x1 += 16;
                    y1 += 8;
                }

                let len = Math.sqrt((x1 - x2)**2 + (y1-y2)**2), new_len = Math.max(len*0.5, len - 16);
                x2 = x1 + new_len/len*(x2-x1);
                y2 = y1 + new_len/len*(y2-y1);
                
                let lineElem = document.createElementNS(SVG_SPEC, "line");
                lineElem.classList.add("order", "move");
                lineElem.setAttribute("x1", computeX(x1 / posData.width)+"");
                lineElem.setAttribute("y1", computeY(y1 / posData.height)+"");
                lineElem.setAttribute("x2", computeX(x2 / posData.width)+"");
                lineElem.setAttribute("y2", computeY(y2 / posData.height)+"");

                if (order_status && order_status[prov] != true) {
                    lineElem.classList.add("fail");
                } else if (active_phase && isRetreat(active_phase)) {
                    lineElem.classList.add("retreat");
                }

                svgElem.appendChild(lineElem);

                break;
            }
            case "support_hold": {
                let src = unitLoc(prov);
                let dest = unitLoc(order.target);

                let x1 = posData.provinces[src].x,
                    y1 = posData.provinces[src].y,
                    x2 = posData.provinces[dest].x,
                    y2 = posData.provinces[dest].y;

                let len = Math.sqrt((x1 - x2)**2 + (y1-y2)**2), new_len = Math.max(len*0.5, len - 16);
                x2 = x1 + new_len/len*(x2-x1);
                y2 = y1 + new_len/len*(y2-y1);
                
                let lineElem = document.createElementNS(SVG_SPEC, "line");
                lineElem.classList.add("order", "support");
                lineElem.setAttribute("x1", computeX(x1 / posData.width)+"");
                lineElem.setAttribute("y1", computeY(y1 / posData.height)+"");
                lineElem.setAttribute("x2", computeX(x2 / posData.width)+"");
                lineElem.setAttribute("y2", computeY(y2 / posData.height)+"");

                if (order_status && order_status[prov] != true) {
                    lineElem.classList.add("fail");
                }

                svgElem.appendChild(lineElem);

                break;
            }
            case "hold": {
                if (active_phase && isRetreat(active_phase) && all_mvmt_info[prevPhase(active_phase)] && prov in all_mvmt_info[prevPhase(active_phase)].retreats) {
                    let src = unitLoc(prov, all_mvmt_info[prevPhase(active_phase)].retreats[prov].src);
                    let [x, y] = [posData.provinces[src].x, posData.provinces[src].y];
                    if (all_states[active_phase].units[prov]) {
                        x += 16; y += 8;
                    }
                    let disband = createDisband(x, y);
                    disband.classList.add("order");
                    break;
                }
                let src = unitLoc(prov);
                let x1 = posData.provinces[src].x,
                    y1 = posData.provinces[src].y;

                let circle = document.createElementNS(SVG_SPEC, "circle");
                circle.setAttribute("cx", "" + computeX(x1 / posData.width));
                circle.setAttribute("cy", "" + computeY(y1 / posData.height));
                circle.setAttribute("r" , "" + (28 / posData.width * svgElem.viewBox.baseVal.height));
                circle.classList.add("order", "hold");

                if (order_status && order_status[prov] != true) {
                    circle.classList.add("retreat");
                }

                svgElem.appendChild(circle);
                break;
            }
            case "support_move":
            case "convoy":
                let provLoc = unitLoc(prov);
                let src = unitLoc(order.src);
                let dest = order.dest;
                if (active_orders[order.src] && active_orders[order.src].type == "move" && (active_orders[order.src] as MoveOrder).dest[1]) {
                    dest += "-" + (active_orders[order.src] as MoveOrder).dest[1];
                }
                console.log(dest, active_orders);

                let x0 = posData.provinces[provLoc].x,
                    y0 = posData.provinces[provLoc].y,
                    x1 = posData.provinces[src].x,
                    y1 = posData.provinces[src].y,
                    x2 = posData.provinces[dest].x,
                    y2 = posData.provinces[dest].y;

                let len = Math.sqrt((x1 - x2)**2 + (y1-y2)**2), new_len = Math.max(len*0.5, len - 16);
                x2 = x1 + new_len/len*(x2-x1);
                y2 = y1 + new_len/len*(y2-y1);

                x0 = computeX(x0/posData.width);
                x1 = computeX(x1/posData.width);
                x2 = computeX(x2/posData.width);
                y0 = computeY(y0/posData.height);
                y1 = computeY(y1/posData.height);
                y2 = computeY(y2/posData.height);

                let elem = document.createElementNS(SVG_SPEC, "path");
                elem.classList.add("order");
                elem.classList.add(order.type == "convoy" ? "convoy" : "support");
                elem.setAttribute("d", `M ${x0} ${y0} C ${x1} ${y1}, ${x1} ${y1}, ${x2} ${y2}`);

                if (order_status && order_status[prov] != true) {
                    elem.classList.add("fail");
                }

                svgElem.appendChild(elem);

                break;
            }
        }
    }

    let all_states: Record<GamePhaseYear, MapState> = {}; // always exists
    let all_orders: Record<GamePhaseYear, Orders> = {};
    let all_mvmt_info: Record<GamePhaseYear, MvmtInfo> = {};
    let all_builds: Record<GamePhaseYear, Builds> = {};

    let units: Record<string, Unit> = {};
    $: {
        if (active_phase && all_states[active_phase]) {
            units = all_states[active_phase].units;
//            console.log("units", units);
        } else if (metaData) {
            units = metaData.starting_state.units;
        } else {
            units = {};
        }
    }

    //ownership
    $: {
        if (active_phase && all_states[active_phase] && metaData) {
            let ownership = all_states[active_phase].ownership;
            let units = all_states[active_phase].units;
            for (let prov in adjData.provinces) {
                if (adjData.provinces[prov].is_sea) continue;
                
                let elem = document.getElementById(prov);
                if (!elem) continue;

                let scElem = document.getElementById("sc-" + prov);

                let power = ownership[prov];
                if (!metaData.provinces[prov].is_sc && units[prov]) {
                    power = unitNatl(units[prov]);
                }

                if (power) {
                    let powerInfo = metaData.powers[power];
                    elem.style.fill = powerInfo.tile_color;
                    if (scElem) {
                        scElem.style.fill = powerInfo.sc_color;
                    }
                } else {
                    elem.style.fill = metaData.data.land_color;
                    if (scElem) {
                        scElem.style.fill = metaData.data.land_color;
                    }
                }
            }
        }
    }

    let svgElem: SVGSVGElement;
    function computeX(ratio: number) : number {
        return ratio*svgElem.viewBox.baseVal.width + svgElem.viewBox.baseVal.x;
    }
    function computeY(ratio: number) : number {
        return ratio*svgElem.viewBox.baseVal.height + svgElem.viewBox.baseVal.y;
    }

    function createUnit(type: "army" | "fleet", power: string, target_x: number, target_y: number) : SVGGraphicsElement {
        let svgBox = svgElem.getBoundingClientRect();

        let prototypeElem: SVGGraphicsElement = document.getElementById(power + "-" + type) as any;
        let cloneElem: SVGGraphicsElement = prototypeElem.cloneNode(true) as any;

        cloneElem.style.display = "";
        cloneElem.removeAttribute("id");
        cloneElem.classList.add("unit");

        let display = prototypeElem.style.display;
        prototypeElem.style.display = "";
        let bbox = prototypeElem.getBoundingClientRect();
        prototypeElem.style.display = display;
        let x = bbox.left + bbox.width/2 - svgBox.left;
        let y = bbox.top + bbox.height/2 - svgBox.top;

        let px = computeX(x / svgBox.width),
            py = computeY(y / svgBox.height);

        let tx = computeX(target_x / posData.width),
            ty = computeY(target_y / posData.height);

        let transform = svgElem.createSVGTransform();
        transform.setTranslate((tx-px), (ty-py));
        cloneElem.transform.baseVal.insertItemBefore(transform, 0);
        prototypeElem.parentNode?.appendChild(cloneElem);

        return cloneElem;
    }

    // draw units
    $: {
        let unitElems = Array.from(document.getElementsByClassName("unit"));
        for (let unit of unitElems) {
            unit.remove();
        }

        if (svgElem) {
            for (let prov in units) {
                let pos: {x : number, y: number};
                if (units[prov].type == "army") {
                    pos = posData.provinces[prov];
                } else {
                    pos = posData.provinces[prov + (units[prov].data[1] ? "-" + units[prov].data[1] : "")];
                }

                createUnit(units[prov].type, unitNatl(units[prov]), pos.x, pos.y);
            }

            if (active_phase && isRetreat(active_phase) && all_mvmt_info[prevPhase(active_phase)]) {
                console.log("drawing retreats", all_mvmt_info[prevPhase(active_phase)].retreats);
                for (let prov in all_mvmt_info[prevPhase(active_phase)].retreats) {
                    let unit =  all_mvmt_info[prevPhase(active_phase)].retreats[prov].src;
                    let pos: {x : number, y: number};
                    if (unit.type == "army") {
                        pos = posData.provinces[prov];
                    } else {
                        pos = posData.provinces[prov + (unit.data[1] ? "-" + unit.data[1] : "")];
                    }

                    if (units[prov]) {
                        pos = Object.assign({}, pos);
                        pos.x += 16;
                        pos.y += 8;
                    }

                    let elem = createUnit(unit.type, unitNatl(unit), pos.x, pos.y);
                    elem.classList.add("retreat");
                }
            }
        }
    }

    function maxPhase(p1: GamePhaseYear, p2: GamePhaseYear) : GamePhaseYear {
        let p1year = p1.split("-")[1] as any | 0;
        let p2year = p1.split("-")[1] as any | 0;
        let p1phase = p1.split("-")[0];
        let p2phase = p1.split("-")[0];

        if (p1year < p2year) return p2;
        if (p2year < p1year) return p1;

        let phases = ["spring", "spring_retreat" ,"fall", "fall_retreat", "winter"];
        let q1 = phases.indexOf(p1phase),
            q2 = phases.indexOf(p2phase);
        if (q1 < q2) return p2;
        if (q2 < q1) return p1;
        return p1;
    }

    let ws: WebSocket | null = null;
    $: if (ws != null && ws.readyState == WebSocket.OPEN) {
        ws.send(JSON.stringify({type : "orders", orders: current_orders}));
    }
    $: if (ws != null && ws.readyState == WebSocket.OPEN) {
        ws.send(JSON.stringify({type : "builds", builds: current_builds}));
    }

    async function init() {
        let game_id = location.pathname.split("/")[2];
        let game_info: GameMeta = JSON.parse(await (await fetch("/games/" + game_id + "/meta.json")).text());

        let mapSvg = await (await fetch("/variants/" + game_info.variant + "/map.svg")).text();
        mapDiv.innerHTML = mapSvg;

        adjData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/adj.json")).text());
        posData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/pos.json")).text());
        metaData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/meta.json")).text());

        svgElem = mapDiv.querySelector("svg") as SVGSVGElement;

        // add necessary things for styling
        svgElem.innerHTML += `  <defs>
    <!-- A marker to be used as an arrowhead -->
    <marker
      id="arrow"
      viewBox="0 0 10 10"
      refX="5"
      refY="5"
      markerWidth="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      markerHeight="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#000" />
    </marker>
    <marker
      id="arrow-fail"
      viewBox="0 0 10 10"
      refX="5"
      refY="5"
      markerWidth="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      markerHeight="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="arrow-fail" />
    </marker>
    <marker
      id="arrow-retreat"
      viewBox="0 0 10 10"
      refX="5"
      refY="5"
      markerWidth="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      markerHeight="${16 / posData.width * svgElem.viewBox.baseVal.width}"
      orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="arrow-retreat" />
    </marker>
  </defs>`;
        var style=document.createElement("style");
        style.textContent = `
        .build { stroke-dasharray: ${8 * svgElem.viewBox.baseVal.width / posData.width} !important;
            stroke-width:  ${8 * svgElem.viewBox.baseVal.width / posData.width} !important; }
        .order, .disband line { stroke-width: ${4 * svgElem.viewBox.baseVal.width / posData.width} !important; }
        .support { stroke-dasharray: ${4 * svgElem.viewBox.baseVal.width / posData.width} }
        .convoy { stroke-dasharray: ${16 * svgElem.viewBox.baseVal.width / posData.width} ${4 * svgElem.viewBox.baseVal.width / posData.width} }
        `;
        document.getElementsByTagName('head')[0].appendChild(style);

        for (let province in adjData.provinces) {
            for (let coast of [""].concat(adjData.provinces[province].coasts)) {
                let name = province + (coast ? "-" + coast : "");

                let tileElem = svgElem.querySelector("#" + name);
                if (tileElem) {
                    tileElem.classList.add("tile");
                } else {
                    let x = computeX(posData.provinces[name].x / posData.width); 
                    let y = computeY(posData.provinces[name].y / posData.height);
                    let circleElem : SVGCircleElement = document.createElementNS(SVG_SPEC, "circle");
                    circleElem.id = province;
                    circleElem.setAttribute("cx", x+"");
                    circleElem.setAttribute("cy", y+"");
                    circleElem.setAttribute("r", String(24* svgElem.viewBox.baseVal.height / posData.height));
                    circleElem.classList.add("tile", "added");
                    if (coast != "") {
                        circleElem.classList.add("coast");
                    }
                    circleElem.id = name;

                    svgElem.appendChild(circleElem);
                    tileElem = circleElem;
                }

                if (adjData.provinces[province].is_sea) {
                    tileElem.classList.add("sea");
                }
                if (coast) {
                    tileElem.classList.add("coast");
                }
                if (coast == "" && adjData.provinces[province].coasts.length != 0) {
                    tileElem.classList.add("has-coast");
                }
                if (adjData.fleet_adj.find(c => c[0][0] == province) == null) {
                    tileElem.classList.add("landlocked");
                }

                tileElem.addEventListener("click", (evt) => {
                    if (current_phase == null || current_phase != active_phase) {
                        return;
                    }

                    if (isRetreat(current_phase) && all_mvmt_info[prevPhase(current_phase)]) {
                        console.log("retreat");

                        let retreatInfo = all_mvmt_info[prevPhase(current_phase)].retreats;
                        order_mode = "move";
                        if (active_prov == null) {
                            if (retreatInfo[province] && unitNatl(retreatInfo[province].src) == mePower) {
                                active_prov = province;
                            }
                        } else {
                            if (active_prov == province) {
                                current_orders[active_prov] = { type: "hold" };   
                                active_prov = active_prov2 = null;
                            } else if (retreatInfo[active_prov] && retreatInfo[active_prov].dest.filter(([p, c]) => p == province && c == coast).length != 0) {
                                current_orders[active_prov] = {
                                    type: "move",
                                    dest: [province, coast]
                                };   
                                active_prov = null;
                            } else if (retreatInfo[active_prov]) {
                                console.log(retreatInfo[active_prov].dest.filter(([p, c]) => p == province && c == coast), retreatInfo[active_prov].dest);
                            }
                        }
                    } else if (!isBuild(current_phase)) {
                        if (active_prov == null) {
                            if (units && units[province] && unitNatl(units[province]) == mePower) {
                                active_prov = province;
                            }
                        } else if (active_prov2 == null && (keydown.s || keydown.c)) {
                            active_prov2 = province;
                            order_mode = keydown.s ? "support" : "convoy";
                        } else {
                            if (order_mode == "move") {
                                if (active_prov != province) {
                                    current_orders[active_prov] = {
                                        type: "move",
                                        dest: [province, coast]
                                    };
                                } else {
                                    current_orders[active_prov] = { type: "hold" };
                                }
                            } else if ((order_mode == "convoy" || order_mode == "support") && active_prov2 && active_prov2 != province) {
                                current_orders[active_prov] = {
                                    type: order_mode == "convoy" ? "convoy" : "support_move",
                                    src: active_prov2,
                                    dest: province
                                };

                                if (units && units[active_prov2] && unitNatl(units[active_prov2]) == mePower) {
                                    current_orders[active_prov2] = {
                                        type: "move",
                                        dest: [province, coast]
                                    };
                                }
                            } else if (order_mode == "support" && active_prov2 && active_prov2 == province) {
                                current_orders[active_prov] = {
                                    type: "support_hold",
                                    target: province
                                };

                                if ((!current_orders[province]) || current_orders[province].type == "move") {
                                    current_orders[province] = { type: "hold" };
                                }
                            }

                            active_prov = active_prov2 = null;
                            order_mode = "move";
                        }
                    } else if (isBuild(current_phase) && all_states[current_phase] && units) {
                        let n_units = Object.values(units).filter(u => unitNatl(u) == mePower).length;
                        let n_supply = Object.values(all_states[current_phase].ownership).filter(p => p == mePower).length;
                        let n_builds = Math.abs(n_units - n_supply);

                        if (n_supply < n_units && units[province]) {
                            if (!current_builds[province]) {
                                if (Object.keys(current_builds).length >= n_builds) {
                                    return;
                                }
                                current_builds[province] = { type: "disband" };
                            } else {
                                delete current_builds[province];
                                current_builds = current_builds;
                            }
                        } else if (n_supply > n_units && !units[province] && all_states[current_phase].ownership[province] == mePower) {
                            if (!current_builds[province]) {
                                if (Object.keys(current_builds).length >= n_builds) {
                                    return;
                                }
                                if (metaData.provinces[province].home_sc != mePower || all_states[current_phase].ownership[province] != mePower) {
                                    console.log(province, metaData.provinces[province].home_sc);
                                    return;
                                }
                                if (keydown.f) {
                                    current_builds[province] = { type: "fleet", coast: coast };
                                } else if (keydown.a) {
                                    current_builds[province] = { type: "army" };
                                }
                            } else {
                                delete current_builds[province];
                                current_builds = current_builds;
                            }
                        }
                    }
                });
            }
        }

        let unitElems = Array.from(document.querySelectorAll("[id^=fleet-]")).
            concat(Array.from(document.querySelectorAll("[id^=army-]")));
        for (let unit of unitElems) {
            unit.classList.add("unit");
        }
        units = units;

        ws = new WebSocket("/games/" + game_id + "/ws");
        ws.onopen = () => {
            let token = new Map(document.cookie.split(";").map(s => s.trim().split("=")) as any).get("token");
            ws.send(JSON.stringify({ type: "auth", token: token }));
        }
        ws.onmessage = (ev) => {
            let msg: Message = JSON.parse(ev.data);
            switch (msg.type) {
            case "game_info":
                mePower = msg.power;
                break;
            case "update_players":
                playerList = msg.players;
                break;
            case "map_state": {
                let phase: GamePhaseYear = `${msg.phase}-${msg.year}`;
                all_states[phase] = msg.state;
                break;
            }
            case "phase": {
                let phase: GamePhaseYear = `${msg.phase}-${msg.year}`;
                all_states[phase] = msg.state;
                if (!active_phase) {
                    active_phase = phase;
                }
                current_orders = current_builds = {};
                current_phase = phase;
                adjTime = msg.adj_time;
                break;
            }
            case "error": {
                console.error(msg.msg);
                break;
            }
            case "movement_adj": {
                let phase: GamePhaseYear = `${msg.phase}-${msg.year}`;
                all_orders[phase] = msg.orders;
                all_mvmt_info[phase] = all_mvmt_info[phase] || {
                    retreat_orders: {}
                };
                all_mvmt_info[phase].order_status = msg.order_status;
                all_mvmt_info[phase].retreats = msg.retreats;
                break;
            }
            case "build_adj": {
                let phase: GamePhaseYear = `${msg.phase}-${msg.year}`;
                all_builds[phase] = msg.builds;
                break;
            }
            case "retreat_adj": {
                let phase: GamePhaseYear = `${msg.phase}-${msg.year}`;
                all_orders[phase] = msg.orders;
                break;
            }
            }
        };
    }

    function formatDuration(millis: number) : string {
        let raw_secs = Math.floor(millis / 1000);
        let min = Math.floor(raw_secs / 60);
        let secs = raw_secs %60;
        return min ? min + "m" + secs + "s" : secs + "s";
    }

    let nowDate :number = Date.now();
    setInterval(() => {
        nowDate = Date.now();
    }, 1000);

    let keydown: Record<string, boolean> = {};
    function keydownlogger(ev: KeyboardEvent) {
        keydown[ev.key] = true;
    }
    function keyuplogger(ev: KeyboardEvent) {
        keydown[ev.key] = false;
    }
</script> 

<style>
    :global(.tile) {
        cursor: pointer;
    }
    :global(.tile.added) {
        z-index: 10;
        fill: rgba(0,0,0,0.075);
    }
    :global(.tile.added.prov-active) {
        fill: rgba(0,0,0,0.2);
    }
    :global(.tile.added.prov-active-2) {
        fill: rgba(255,0,0,0.2);
    }
    :global(.tile.prov-active) {
        filter: brightness(80%);
    }
    :global(.tile.prov-active-2) {
        filter: brightness(80%) hue-rotate(90deg);  }

    .build-army :global(.coast), .build-army :global(.sea), .build-fleet :global(.landlocked), .build-fleet :global(.has-coast),
    .move-army :global(.coast), .move-army :global(.sea), .move-fleet :global(.landlocked), .move-fleet :global(.has-coast),
    .no-touch :global(.tile), .move-none :global(.coast) {
        pointer-events: none; }
    .build-army :global(.coast.added), .build-army :global(.sea.added), .build-fleet :global(.landlocked.added), .build-fleet :global(.has-coast.added),
    .move-army :global(.coast.added), .move-army :global(.sea.added), .move-fleet :global(.landlocked.added), .move-fleet :global(.has-coast.added),
    .no-touch  :global(.tile.added), .move-none :global(.coast) {
        display: none; }

    .panel {
        position: fixed;
        top: 16px;
        left: 16px;
        background: hsl(200, 15%, 10%);
        padding: 16px;
    }
    .panel h3 {
        margin-top: 0;
        text-align: center;
        font-size: 18px;
    }

    #players {
        padding: 0;
        padding-bottom: 16px;
        width: 320px; }
    #players h3 {
        padding: 16px;
        padding-bottom: 8px;
        margin: 0; }
    .player {
        display: flex;
        align-items: center;
        flex-direction: row; }
    .player.me { color: #000; }
    .power-color { width: 16px; align-self: stretch; display: inline-block; }
    .power-name {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        justify-content: center;
        padding: 4px 16px;
        flex-grow: 1; }
    .power-country { font-size: 16px; }
    .power-user { font-size: 13px; margin-top: 0; }
    .player > span:not(.power-color) {
        padding: 8px 16px;
        margin: 0;
        padding-bottom: 8px;
        text-align: center;
        font-size: 16px;
    }

    #phase-panel {
        left: 50%;
        transform: translateX(-50%);
        display: flex;
        flex-direction: row;
        min-width: 256px;
        padding: 0;
    }
    #phase-panel button {
        padding: 8px 16px;
        font-size: 32px;
        background: transparent; }
    #phase-panel button:active {
        background: rgba(0,0,0,0.5); }
    #phase-panel > div {
        text-align: center;
        flex-grow: 1;
        margin: 0 4px; }
    #phase {
        font-size: 20px; }
    

    :global(body) { background: #fff; }

    :global(.order) { fill: transparent; 
        pointer-events: none;
        z-index: 20; }
    :global(.hold.order) { 
        --arrow-color: #000;
        stroke: #000;
    }
    :global(.move.order) {
        stroke: #000;
        marker-end: url(#arrow);
    }
    :global(.convoy.order) {
        z-index: 21;
        stroke: hsl(240, 50%, 35%); }
    :global(.support) {
        z-index: 21;
        stroke: #000;
    }
    :global(.order.fail) {
        marker-end: url(#arrow-fail); 
        stroke: hsl(0, 65%, 50%); }
    :global(.arrow-fail) {
        fill: hsl(0, 65%, 50%); 
    }
    :global(.unit) {
        z-index: 15;
    }
    :global(.unit.retreat) {
        filter: 
            drop-shadow(1px 0 1px hsl(35, 100%, 50%))
            drop-shadow(-1px 0 1px hsl(35, 100%, 50%))
            drop-shadow(0 1px 1px hsl(35, 100%, 50%))
            drop-shadow(0 -1px 1px hsl(35, 100%, 50%))
            ; }
    :global(.order.retreat) {
        stroke: hsl(35, 75%, 50%);
        marker-end: url(#arrow-retreat); }
    :global(.arrow-retreat) { 
        fill: hsl(35, 75%, 50%); }
    :global(text), :global([id^=sc-]), :global(.unit) {
        pointer-events: none; }

    :global(.disband line) {
        z-index: 21;
        stroke: hsl(0, 65%, 50%); }
    :global(.build), :global(.disband) {
        pointer-events: none; }
    :global(.build) {
        stroke-linejoin: round; }

    #status-panel {
        padding: 0;
        position: fixed;
        bottom: 16px; left: 16px;
        top: auto;
        display: flex;
        flex-direction: row;
        width: 256px;
    }
    #status-panel > div {
        flex-grow: 1;
        padding: 8px 16px;
        text-align: center;
    }
    #status-panel > div.active {
        background: hsl(330, 50%, 45%);
    }

    #map { 
        display: flex;
        height: 100vh;
        align-items: center;
        justify-content: center;
    }
</style>

<div id="map" bind:this={mapDiv} 
    class:no-touch={!current_phase || current_phase != active_phase}
    class:move-none={current_phase && !isBuild(current_phase) && active_prov == null}
    class:move-army={current_phase && !isBuild(current_phase) && active_prov && units[active_prov2 || active_prov] && units[active_prov2 || active_prov].type == "army"}
    class:move-fleet={current_phase && !isBuild(current_phase) && active_prov && units[active_prov2 || active_prov] && units[active_prov2 || active_prov].type == "fleet"}
    class:build-army={current_phase && isBuild(current_phase) && active_phase == current_phase && keydown.a}
    class:build-fleet={current_phase && isBuild(current_phase) && active_phase == current_phase && keydown.f}></div>

<div class="panel" id="players">
    <h3>Players</h3>
    {#each playerList as player}
        {@const n_supply = active_phase && all_states[active_phase] && Object.values(all_states[active_phase].ownership).filter(p => p == player[0]).length || 0}
        <div class="player" class:me={mePower && player[0] == mePower} style:background={mePower && player[0] == mePower ? metaData.powers[mePower].tile_color : ""}>
            {#if player[0]}
            <span style={"background:" + metaData.powers[player[0]].sc_color} class="power-color"></span>
            {/if}
            <div class="power-name">
                <div class="power-country">{player[0] ? (metaData.powers[player[0]].name || player[0]) : player[1]}</div>
                {#if player[0]}<div class="power-user">{player[1]}</div>{/if}
            </div>

            {#if player[0]}
                {#if active_phase && isBuild(active_phase)}
                {@const n_units = Object.values(units).filter(u => unitNatl(u) == player[0]).length}
                    <span class="power-builds">{n_supply - n_units > 0 ? "+" : ""}{n_supply - n_units}</span>
                {/if}

                <span class="power-sc">{n_supply}</span>
            {/if}
        </div>
    {/each}
</div>

{#if active_phase}
{@const year = Number(active_phase.split("-")[1])}
<div class="panel" id="phase-panel">
    <button on:click={() => { if (active_phase) active_phase = prevNonemptyPhase(active_phase, all_mvmt_info) }}>&lt;</button>
    <div style="display:flex;flex-direction:column;justify-content:center">
        <div id="phase">
            {PHASES[active_phase.split("-")[0]]} {"'" + (year < 10 ? "0" + year : year)} {isRetreat(active_phase) ? "retreats" : ""}
        </div>
        {#if active_phase == current_phase}
        <div id="adj-time">{formatDuration(adjTime - nowDate)} left</div>
        {/if}
    </div>
    <button on:click={() => { if(active_phase) active_phase = nextNonemptyPhase(active_phase, all_mvmt_info) }}>&gt;</button>
</div>
{/if}

{#if active_phase == current_phase && current_phase}
<div class="panel" id="status-panel">
    {#if !isBuild(active_phase)}
    {@const mode = (keydown.c || order_mode == "convoy") ? "convoy" : (keydown.s || order_mode == "support") ? "support" : "move"}
    <div class:active={mode == "move"}>Move</div>
    <div class:active={mode == 'convoy'}>Convoy</div>
    <div class:active={mode == 'support'}>Support</div>
    {/if}
    {#if isBuild(active_phase)}
    <div class:active={keydown.a}>Army</div>
    <div class:active={keydown.f}>Fleet</div>
    {/if}
</div>
{/if}

<svelte:window on:load={init} on:keydown={keydownlogger} on:keyup={keyuplogger} />