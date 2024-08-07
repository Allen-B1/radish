// base
export type Unit = { type: "army", data: string } | { type: "fleet", data: [string, string] };

export function unitNatl(unit: Unit) : string {
    return unit.type == "army" ? unit.data : unit.data[0];
}
export interface Province {
    is_sea: boolean,
    coasts: string[]
}

export interface MapState {
    units: Record<string, Unit>,
    ownership: Record<string, string>,
}

export type FleetLoc = [string, string];
export type ArmyLoc = string;

export type MoveOrder = { type: "move", dest: FleetLoc };
export type Order = MoveOrder
    | { type : "hold" }
    | { type : "convoy", src: string, dest: string } 
    | { type : "support_move", src: string, dest: string } 
    | { type : "support_hold", target : string }
    | { type : "core" };
export type Orders = Record<string, Order>;

// utils
export interface GameMeta {
    name: string,
    press: PressType,
    variant: string,
};

export interface AdjData {
    provinces: Record<string, Province>,
    fleet_adj: Array<[[string, string], [string, string]]>
    army_adj: Array<[string, string]>,
}

export interface PowerMeta {
    name: string,
    tile_color: string,
    sc_color: string,
}

export interface ProvinceMeta {
    name: string,
    is_sc: boolean,
    home_sc: string
}

export interface MapMeta {
    name: string,
    author: string,
    powers: Record<string, PowerMeta>,
    provinces: Record<string, ProvinceMeta>,
    data: {
        land_color: string,
        id: string,
    }
}

export interface PosData {
    provinces: Record<string, { x : number, y : number }>,
    width: number,
    height: number
}

export interface RetreatOptions {
    src: Unit,
    dest: Array<[string, string]>
}

// server

export type GamePhase = "fall" | "fall_retreat" | "spring" | "spring_retreat" | "winter";

export const PHASES = {
    "fall": "Fall",
    "fall_retreat": "Fall (r)",
    "spring": "Spring",
    "spring_retreat": "Spring (r)",
    "winter": "Winter"
};
export type GamePhaseYear = `${GamePhase}-${number}`;

export function nextPhase(gp: GamePhaseYear): GamePhaseYear {
    let parts = gp.split("-");
    let year = Number(parts[1]);
    switch (parts[0]) {
        case "spring": return `spring_retreat-${year}`;
        case "spring_retreat": return `fall-${year}`;
        case "fall": return `fall_retreat-${year}`;
        case "fall_retreat": return `winter-${year}`;
        case "winter": return `spring-${year+1}`;
    }

    throw new Error("unknown phase: " + gp);
}
export function prevPhase(gp: GamePhaseYear): GamePhaseYear {
    let parts = gp.split("-");
    let year = Number(parts[1]);
    switch (parts[0]) {
        case "spring": return `winter-${year-1}`;
        case "spring_retreat": return `spring-${year}`;
        case "fall": return `spring_retreat-${year}`;
        case "fall_retreat": return `fall-${year}`;
        case "winter": return `fall_retreat-${year}`;
    }

    throw new Error("unknown phase: " + gp);
}

export type PressType = "full" | "rulebook" | "public" | "gunboat";

export type Builds = Record<string, Unit>;

export interface MvmtInfo {
    order_status: Record<string, boolean>,
    retreats: Set<string>,
}

export function isBuild(gp: GamePhaseYear): boolean {
    let phase = gp.split("-")[0];
    return phase == "winter"; 
}
export function isRetreat(gp: GamePhaseYear): boolean {
    let phase = gp.split("-")[0];
    return phase.endsWith("retreat");
}