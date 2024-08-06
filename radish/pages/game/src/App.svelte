
<script lang="ts">
    const SVG_SPEC = 'http://www.w3.org/2000/svg';

    type PressType = "full" | "rulebook" | "public" | "gunboat";

    interface GameMeta {
        name: string,
        press: PressType,
        variant: string,
    };

    interface Province {
        is_sea: boolean,
        coasts: string[]
    }

    interface AdjData {
        provinces: Record<string, Province>,
        fleet_adj: Array<[[string, string], [string, string]]>
        army_adj: Array<[string, string]>,
    }

    interface MapMeta {
        name: string,
        author: string,
        powers: Record<string, PowerMeta>,
        provinces: Record<string, ProvinceMeta>,
        data: {
            land_color: string,
            id: string,
        }
    }

    interface PosData {
        provinces: Record<string, { x : number, y : number }>,
        width: number,
        height: number
    }

    let mapDiv: HTMLDivElement;

    function handleClick(province: string) {

    }


    let adjData: AdjData;
    let posData: PosData;
    let metaData: MapMeta;

    function sleep(ms: number) {
        return new Promise((res) => {
            setTimeout(res, ms);
        });
    }

    async function init() {
        let game_id = location.pathname.split("/")[2];
        let game_info: GameMeta = JSON.parse(await (await fetch("/games/" + game_id + "/meta.json")).text());

        let mapSvg = await (await fetch("/variants/" + game_info.variant + "/map.svg")).text();
        mapDiv.innerHTML = mapSvg;

        adjData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/adj.json")).text());
        posData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/pos.json")).text());
        metaData = JSON.parse(await (await fetch("/variants/" + game_info.variant + "/meta.json")).text());

        let svgElem = mapDiv.querySelector("svg") as SVGSVGElement;
        for (let province in adjData.provinces) {
            let tileElem = svgElem.querySelector("#" + province);
            if (tileElem) {
                tileElem.classList.add("tile");
            } else {
                let hr = svgElem.viewBox.baseVal.width / posData.width;
                let vr = svgElem.viewBox.baseVal.height / posData.height;
                console.log(hr);
                let x = hr * posData.provinces[province].x; 
                let y = vr * posData.provinces[province].y;
                let circleElem : SVGCircleElement = document.createElementNS(SVG_SPEC, "circle");
                circleElem.id = province;
                circleElem.setAttribute("cx", x+"");
                circleElem.setAttribute("cy", y+"");
                circleElem.setAttribute("r", 24*vr);
                circleElem.style.strokeWidth = 1*vr;
                circleElem.classList.add("tile", "added");
                circleElem.style.zIndex = "9";

                svgElem.appendChild(circleElem);
                tileElem = circleElem;
            }


            tileElem.addEventListener("click", () => {
                handleClick(province);
            });
        }
    }
</script> 

<style>
    :global(.tile.added) {
        fill: transparent;
        stroke: #000;
    }
    :global(.tile.added):hover {
        fill: rgba(0,0,0,0.8);
    }
    :global(.tile):hover {
        filter: brightness(80%);
        cursor: pointer;
    }
</style>

<div id="map" bind:this={mapDiv}></div>
<svelte:window on:load={init} />