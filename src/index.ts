import * as vscode from 'vscode';
import { FunctionParameterDeclaration, Identifier, Loc, Node, StatementNode, TupleLiteral, TypeCall, TypeReference, TypeTuple, rs } from "jinx-rust";
import { Uri } from 'vscode';

function intersect_safe<T>(a: T[], b: T[]): T[] {
    return Array.from(new Set(b.filter(Set.prototype.has.bind(new Set(a)))));
}

type Adders
    = "addDirect"
    | "addSystem"
    | "addQuery"
    | "addMutQuery"
    | "addEventReader"
    | "addEventWriter"
    | "addRes"
    | "addMutRes"
    | "addWith"
    | "addWithout";

type QueryStorage
    = "event_write"
    | "event_read"
    | "query"
    | "mut_query"
    | "res"
    | "mut_res"
    | "with"
    | "without"
    | "systems";

export class BevyrlyIndex {
    any: Map<string, Set<string>>;
    direct: Map<string, Set<string>>;
    event_write: Map<string, Set<string>>;
    event_read: Map<string, Set<string>>;
    query: Map<string, Set<string>>;
    mut_query: Map<string, Set<string>>;
    res: Map<string, Set<string>>;
    mut_res: Map<string, Set<string>>;
    with: Map<string, Set<string>>;
    without: Map<string, Set<string>>;
    systems: Map<string, Set<string>>;
    locs: Map<string, Loc>;
    isInitialized: boolean;

    constructor() {
        this.any = new Map();
        this.direct = new Map();
        this.event_write = new Map();
        this.event_read = new Map();
        this.query = new Map();
        this.mut_query = new Map();
        this.res = new Map();
        this.mut_res = new Map();
        this.with = new Map();
        this.without = new Map();
        this.systems = new Map();
        this.locs = new Map();
        this.isInitialized = false;
    }

    public toString(): string {
        return this.query.size +
            ", " + this.direct.size +
            ", " + this.mut_query.size +
            ", " + this.event_write.size +
            ", " + this.event_read.size +
            ", " + this.res.size +
            ", " + this.mut_res.size +
            ", " + this.with.size +
            ", " + this.without.size;
    }

    addAny(system: string, c: string) {
        if (!this.any.has(c)) {
            this.any.set(c, new Set());
        }

        this.any.get(c)?.add(system);
    }

    addDirect(system: string, c: string) {
        if (!this.direct.has(c)) {
            this.direct.set(c, new Set());
        }

        this.direct.get(c)?.add(system);
        this.addSystem(system, c);
    }

    addSystem(system: string, c: string) {
        if (!this.systems.has(system)) {
            this.systems.set(system, new Set());
        }

        this.systems.get(system)?.add(c);
        this.addAny(system, c);
    }

    addQueryStorage(system: string, c: string, storage: QueryStorage) {
        if (!this[storage].has(c)) {
            this[storage].set(c, new Set());
        }

        this[storage].get(c)?.add(system);
        this.addSystem(system, c);
    }

    addQuery(system: string, c: string) {
        this.addQueryStorage(system, c, "query");
    }

    addMutQuery(system: string, c: string) {
        this.addQueryStorage(system, c, "mut_query");
    }

    addEventReader(system: string, c: string) {
        this.addQueryStorage(system, c, "event_write");
    }

    addEventWriter(system: string, c: string) {
        this.addQueryStorage(system, c, "event_read");
    }

    addRes(system: string, c: string) {
        this.addQueryStorage(system, c, "res");
    }

    addMutRes(system: string, c: string) {
        this.addQueryStorage(system, c, "mut_res");
    }

    addWith(system: string, c: string) {
        this.addQueryStorage(system, c, "with");
    }

    addWithout(system: string, c: string) {
        this.addQueryStorage(system, c, "without");
    }

    removeSystem(system: string) {
        if (this.systems.has(system)) {
            this.locs.delete(system);
            const dets = this.systems.get(system);
            if (dets) {
                for (const det of dets) {
                    if (this.event_write.has(det)) { this.event_write.get(det)?.delete(system); }
                    if (this.event_read.has(det)) { this.event_read.get(det)?.delete(system); }
                    if (this.query.has(det)) { this.query.get(det)?.delete(system); }
                    if (this.mut_query.has(det)) { this.mut_query.get(det)?.delete(system); }
                    if (this.res.has(det)) { this.res.get(det)?.delete(system); }
                    if (this.mut_res.has(det)) { this.mut_res.get(det)?.delete(system); }
                    if (this.with.has(det)) { this.with.get(det)?.delete(system); }
                    if (this.without.has(det)) { this.without.get(det)?.delete(system); }
                    if (this.direct.has(det)) { this.direct.get(det)?.delete(system); }
                    if (this.any.has(det)) { this.any.get(det)?.delete(system); }
                }
            }
            this.systems.delete(system);
        }
    }

    clear() {
        for (const system of this.systems.keys()) {
            this.removeSystem(system);
        }
    }

    get(s: string): [string[], "short" | "long"] {
        let all_systems = Array.from(this.systems.keys());
        let long_print = s.startsWith(":");
        if (long_print) {
            s = s.slice(1).trim();
        }

        for (const part of s.split(" ")) {
            let ident = part.slice(1);
            let map: Map<string, Set<string>>;
            switch (part.at(0)) {
                case '&': map = this.query; break;
                case '*': map = this.mut_query; break;
                case '>': map = this.event_write; break;
                case '<': map = this.event_read; break;
                case '#': map = this.res; break;
                case '$': map = this.mut_res; break;
                case '+': map = this.with; break;
                case '-': map = this.without; break;
                default: map = this.any; ident = part; break;
            }

            let layer = Array.from(map.keys())
                .filter(key => (key === undefined) ? false : key.includes(ident))
                .flatMap(key => Array.from(map.get(key) ?? []));

            all_systems = intersect_safe(all_systems, layer);
            if (all_systems.length == 0) return [all_systems, "short"];
        }

        return [all_systems, long_print ? "long" : "short"];
    }

    addItem(generics: Set<string>, item: Adders, par: any, system_name: string) {
        let obj;
        if (par.typeAnnotation) {
            obj = par.typeAnnotation;
        } else {
            obj = par;
        }

        if (obj instanceof TypeCall) {
            let name = obj.typeCallee.name;
            if (!new Set(["With", "Without", "Res", "ResMut", "Option", "Query", "Local", "NonSendMut"]).has(name)) {
                this[item](system_name, name);
            }
            for (const sub of obj.typeArguments.values()) {
                if (sub instanceof Identifier) {
                    if (generics.has(sub.name)) continue;
                    this[item](system_name, sub.name);
                } else if (sub instanceof TypeCall) {
                    let typeJson = sub.typeCallee.toJSON();
                    let name = typeJson["name"];
                    let innerType = sub.typeArguments[0];
                    if (!this.recursiveTypeCall(generics, innerType, typeJson, name, system_name)) {
                        //console.log("REC failed: ", sub);
                    }
                }
            }
        } else if (obj instanceof Identifier) {
            this[item](system_name, obj.name);
        }
    }

    recursiveTypeCall(generics: Set<string>, par: any, typeJson: string, name: string, system_name: string): boolean {
        if (name == "Query" || name == "Local") {
            for (const arg of par.typeArguments.values()) {
                if (arg instanceof Identifier) {
                    this.addDirect(system_name, arg.name);
                } else if (arg instanceof TypeTuple) {
                    for (const sub of arg.items.values()) {
                        if (sub instanceof TypeReference) {
                            const ref = sub.typeExpression.toJSON()['name'];
                            if (generics.has(ref)) continue;

                            if (sub.mut) {
                                this.addMutQuery(system_name, ref);
                            } else {
                                this.addQuery(system_name, ref);
                            }
                        } else if (sub instanceof Identifier) {
                            this.addDirect(system_name, sub.name);
                        } else if (sub instanceof TypeCall) {
                            let typeJson = sub.typeCallee.toJSON();
                            let name = typeJson["name"];
                            let innerType = sub.typeArguments[0];
                            if (!this.recursiveTypeCall(generics, innerType, typeJson, name, system_name)) {
                                //console.log("REC failed: ", sub);
                            }
                        }
                    }
                } else if (arg instanceof TypeReference) {
                    const ref = arg.typeExpression.toJSON()['name'];
                    if (generics.has(ref)) continue;

                    if (arg.mut) {
                        this.addMutQuery(system_name, ref);
                    } else {
                        this.addQuery(system_name, ref);
                    }
                } else if (arg instanceof TypeCall) {
                    let typeJson = arg.typeCallee.toJSON();
                    let name = typeJson["name"];
                    if (name == "With") {
                        for (const sub of arg.typeArguments.values()) {
                            if (sub instanceof Identifier) {
                                if (generics.has(sub.name)) continue;
                                this.addWith(system_name, sub.name);
                            } else {
                                console.log("WITH", sub);
                            }
                        }
                    } else if (name == "Without") {
                        for (const sub of arg.typeArguments.values()) {
                            if (sub instanceof Identifier) {
                                if (generics.has(sub.name)) continue;
                                this.addWithout(system_name, sub.name);
                            } else {
                                console.log("WITHOUT", sub);
                            }
                        }
                    }
                } else {
                    console.log("[ERR] Unknown argument type: ", arg);
                }
            }
            return true;
        } else if (name == "Res") {
            this.addItem(generics, "addRes", par, system_name);
            return true;
        } else if (name == "ResMut" || name == "NonSendMut") {
            this.addItem(generics, "addMutRes", par, system_name);
            return true;
        } else if (name == "EventReader") {
            this.addItem(generics, "addEventReader", par, system_name);
            return true;
        } else if (name == "EventWriter") {
            this.addItem(generics, "addEventWriter", par, system_name);
            return true;
        } else if (name == "With") {
            this.addItem(generics, "addWith", par, system_name);
            return true;
        } else if (name == "Without") {
            this.addItem(generics, "addWithout", par, system_name);
            return true;
        } else if (name == "Option") {
            let typeJson = par.typeArguments[0].typeCallee.toJSON();
            let name = typeJson["name"];
            let innerType = par.typeArguments[0];
            if (!this.recursiveTypeCall(generics, innerType, typeJson, name, system_name)) {
                //console.log("REC failed: ", par);
            }
            return true;
        } else { // this is just a direct generic
            this.addItem(generics, "addDirect", par, system_name);
            return false;
        }
    }

    addFunctionNode<T extends Node>(node: T) {
        if (node.nodeType == 38) {
            let generics: Set<string> = new Set();

            const system_name = node.id.name;

            this.locs.set(system_name, node.loc);

            if (node.generics !== undefined) {
                for (const gen of node.generics.values()) {
                    generics.add(gen.id.name.toString());
                }
            }

            if (node.parameters) {
                for (const par of node.parameters.values()) {
                    if (par instanceof FunctionParameterDeclaration) {
                        if (par.typeAnnotation instanceof TypeCall) {
                            let typeJson = par.typeAnnotation.typeCallee.toJSON();
                            let name = typeJson["name"];

                            if (!this.recursiveTypeCall(generics, par.typeAnnotation, typeJson, name, system_name)) {
                                //console.log("Recursion failed with ", par);
                            }
                        } else if (par.typeAnnotation instanceof Identifier) {
                            this.addDirect(system_name, par.typeAnnotation.name);
                        }
                    } else {
                        console.log("Not taking ", par);
                    }
                }
            }
        }
    }
}

export function startBevyrlyIndexing(context: vscode.ExtensionContext, bevyrlyIndex: BevyrlyIndex) {
    bevyrlyIndex.clear();

    if (vscode.workspace.workspaceFolders) {
        // commands
        for (const folder of vscode.workspace.workspaceFolders) {
            let path = Uri.joinPath(folder.uri, "src");
            vscode.workspace.fs.readDirectory(path).then(async (r: [string, vscode.FileType][]) => {
                for (const file of r.values()) {
                    if (file[0].endsWith(".rs")) {
                        const filepath = Uri.joinPath(path, file[0]);
                        await vscode.workspace.openTextDocument(filepath).then((f: vscode.TextDocument) => {
                            let ast = rs.parseFile(f.getText(), { filepath: filepath.toString() }).program.ast;
                            for (const node of ast.values()) {
                                if (node.nodeType == 38) {
                                    bevyrlyIndex.addFunctionNode(node);
                                } else if (node.nodeType == 54) {
                                    for (const sub of node.body.values()) {
                                        if (sub.nodeType == 38) {
                                            bevyrlyIndex.addFunctionNode(sub);
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }).then(_ => {
                bevyrlyIndex.isInitialized = true;
            });
        }
    }
}
