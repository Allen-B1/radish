
<script lang="ts">
    const SVG_SPEC = 'http://www.w3.org/2000/svg';

    import { type AdjData, type PosData, type MapMeta, type GameMeta, type MapState, type Orders,  type MoveOrder, type Builds, type Unit, type GamePhase, type GamePhaseYear, type RetreatOptions, type MvmtInfo, PHASES, prevPhase, nextPhase, isBuild, unitNatl, isRetreat } from './defs';

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

    function unitLoc(prov: string) : string {
        if (active_phase == null) { return prov }

        // can't use units for reasons
        let units = all_states[active_phase].units;
        if (units && units[prov] && units[prov].type == "fleet" && units[prov].data[1] != "") {
            return prov + "-" + units[prov].data[1]
        } else {
            return prov;
        }
    }

    let active_orders: Readonly<Orders> = current_orders;
    $: {
        active_orders = active_phase == current_phase ? current_orders : (all_orders[active_phase] || {});
    }
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
                let dest = order.dest[0] + "" + (order.dest[1] ? "-" + order.dest[1] : "");

                let x1 = posData.provinces[src].x,
                    y1 = posData.provinces[src].y,
                    x2 = posData.provinces[dest].x,
                    y2 = posData.provinces[dest].y;

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

    $: {
        let unitElems = Array.from(document.getElementsByClassName("unit"));
        for (let unit of unitElems) {
            unit.remove();
        }

        if (svgElem) {
            let svgBox = svgElem.getBoundingClientRect();

            for (let prov in units) {
                let posObject: { x : number, y : number };
                let prototypeElem: SVGGraphicsElement;
                if (units[prov].type == "army") {
                    prototypeElem = document.getElementById(units[prov].data + "-army") as any;
                    posObject = posData.provinces[prov]
                } else {
                    prototypeElem = document.getElementById(units[prov].data[0] + "-fleet") as any;
                    posObject = posData.provinces[prov + (units[prov].data[1] ? "-" + units[prov].data[1] : "")];
                }

                let cloneElem: SVGGraphicsElement = prototypeElem.cloneNode(true) as any;

                cloneElem.removeAttribute("id");
                cloneElem.classList.add("unit");

                let bbox = prototypeElem.getBoundingClientRect();
                let x = bbox.left + bbox.width/2 + window.scrollX;
                let y = bbox.top + bbox.height/2 + window.scrollY;

                let px = computeX(x / svgBox.width),
                    py = computeY(y / svgBox.height);

                let tx = computeX(posObject.x / posData.width),
                    ty = computeY(posObject.y / posData.height);

                cloneElem.setAttribute("transform", 
                    "translate(" + (tx-px) + "," + (ty-py) + ") " + 
                    cloneElem.getAttribute("transform")
                );

                prototypeElem.parentNode?.appendChild(cloneElem);
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
        .order { stroke-width: ${4 * svgElem.viewBox.baseVal.width / posData.width} }
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

                tileElem.addEventListener("click", (evt) => {
                    if (current_phase != null && !isBuild(current_phase) && current_phase == active_phase) {
                        if (active_prov == null) {
                            if (units && units[province] && unitNatl(units[province]) == mePower) {
                                active_prov = province;
                            }
                        } else if (active_prov2 == null && (keydown.s || keydown.c) && !isRetreat(current_phase)) {
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
                            } else if (order_mode == "convoy" && active_prov2) {
                                current_orders[active_prov] = {
                                    type: "convoy",
                                    src: active_prov2,
                                    dest: province
                                }
                            } else if (order_mode == "support" && active_prov2) {
                                if (active_prov2 != province) {
                                    current_orders[active_prov] = {
                                        type: "support_move",
                                        src: active_prov2,
                                        dest: province
                                    }
                                } else {
                                    current_orders[active_prov] = {
                                        type: "support_hold",
                                        target: province
                                    };
                                }
                            }

                            active_prov = active_prov2 = null;
                            order_mode = "move";
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
                current_orders = {};
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
                all_mvmt_info[phase].retreats = new Set(Object.keys(msg.retreats));
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
        filter: brightness(80%) hue-rotate(90deg);
    }

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
    }

    .player {
        display: flex;
        align-items: center;
        flex-direction: row; }
    .power-color { width: 8px; height: 32px; display: inline-block; margin-right: 8px; }
    .power-name {
        flex-grow: 1; }
    .me .power-name { font-weight: bold;}

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

    :not(.show-coast) :global(.coast) {
        display: none;
    }

    :global(.order) { fill: transparent; 
        pointer-events: none;
        z-index: 11; }
    :global(.hold.order) { 
        --arrow-color: #000;
        stroke: #000;
    }
    :global(.move.order) {
        stroke: #000;
        marker-end: url(#arrow);
    }
    :global(.convoy.order) { stroke: hsl(240, 50%, 35%); }
    :global(.support) {
        z-index: 12;
        stroke: #000;
    }
    :global(.order.fail) {
        marker-end: url(#arrow-fail); 
        stroke: hsl(0, 75%, 50%); }
    :global(.arrow-fail) {
        fill: hsl(0, 75%, 50%); 
    }
    :global(.order.retreat) {
        stroke: hsl(35, 75%, 50%);
        marker-end: url(#arrow-retreat); }
    :global(.arrow-retreat) { 
        fill: hsl(35, 75%, 50%); }
    :global(text), :global([id^=sc-]), :global(.unit) {
        pointer-events: none;
    }
</style>

<div id="map" bind:this={mapDiv} class:show-coast={order_mode == "move" && current_phase && units && active_prov && units[active_prov] && units[active_prov].type == "fleet"}></div>

<div class="panel" id="players">
    <h3>Players</h3>
    {#each playerList as player}
        <div class="player" class:me={player[0] == mePower}>
            {#if player[0]}
            <span style={"background:" + metaData.powers[player[0]].sc_color} class="power-color"></span>
            {/if}
            <span class="power-name">
                {player[1]}
            </span>
        </div>
    {/each}
</div>

{#if active_phase}
{@const year = Number(active_phase.split("-")[1])}
<div class="panel" id="phase-panel">
    <button on:click={() => { if (active_phase) active_phase = prevPhase(active_phase) }}>&lt;</button>
    <div style="display:flex;flex-direction:column;justify-content:center">
        <div id="phase">
            {PHASES[active_phase.split("-")[0]]} {"'" + (year < 10 ? "0" + year : year)}
        </div>
        {#if active_phase == current_phase}
        <div id="adj-time">{formatDuration(adjTime - nowDate)} left</div>
        {/if}
    </div>
    <button on:click={() => { if(active_phase) active_phase = nextPhase(active_phase) }}>&gt;</button>
</div>
{/if}

<svelte:window on:load={init} on:keydown={keydownlogger} on:keyup={keyuplogger} />