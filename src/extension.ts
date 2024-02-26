import * as vscode from 'vscode';
import { Uri } from 'vscode';
import { BevyrlyIndex, startBevyrlyIndexing } from './index';
import { registerTextDocument } from './text_document';
import { registerNotebookDocument } from './notebook_document';
import { Loc } from 'jinx-rust';

let bevyrlyIndex: BevyrlyIndex = new BevyrlyIndex();

export function expandLinkFromName(bevyrlyIndex: BevyrlyIndex, system: string): [Loc, string] | undefined {
    let loc = bevyrlyIndex.locs.get(system);
    if (loc) {
        const start = loc.src.l(loc[0]) + 1;
        return [loc, vscode.Uri.parse(loc.src.filepath?.replace("file:///", "") ?? "").path.split('/src/').pop() + ":" + start];
    }

    return undefined;
}

export function expandSystemFromName(bevyrlyIndex: BevyrlyIndex, system: string, withLink: boolean = true): string {
    let content = "";
    let loc = bevyrlyIndex.locs.get(system);
    if (loc) {
        if (withLink) {
            const start = loc.src.l(loc[0]);
            const end = loc.src.l(loc[1]);
            content += "\n/* " + vscode.Uri.parse(loc.src.filepath?.replace("file:///", "") ?? "").path.split('/src/').pop() + ":" + start + "-" + end + " */\n";
        }
        content += loc.getText();
    }

    return content;
}

export function activate(context: vscode.ExtensionContext) {
    context.subscriptions.push(vscode.commands.registerCommand('bevyrly.start', () => {
        startBevyrlyIndexing(context, bevyrlyIndex);
    }));

    registerTextDocument(context, bevyrlyIndex);
    registerNotebookDocument(context, bevyrlyIndex);
}
