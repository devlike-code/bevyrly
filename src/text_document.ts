import * as vscode from 'vscode';
import { BevyrlyIndex } from '.';
import { expandSystemFromName } from './extension';

async function showBevyrlyResultsAsTextDocument(result: string) {
    let doc = await vscode.workspace.openTextDocument(vscode.Uri.parse(encodeURI(result)).with({ scheme: 'bevyrly' }));
    vscode.languages.setTextDocumentLanguage(doc, "rust");
    await vscode.window.showTextDocument(doc, { preview: false });
}

let virtualTextProvider: vscode.TextDocumentContentProvider | null = null;

function createVirtualTextProvider(bevyrlyIndex: BevyrlyIndex): vscode.TextDocumentContentProvider {
    return new (class implements vscode.TextDocumentContentProvider {
        onDidChange?: vscode.Event<vscode.Uri> | undefined;

        provideTextDocumentContent(uri: vscode.Uri, token: vscode.CancellationToken): vscode.ProviderResult<string> {
            let search = uri.path.slice(1);
            let content = "";
            for (const system of bevyrlyIndex.get(search)) {
                content += expandSystemFromName(bevyrlyIndex, system);
            }

            return content;
        }
    })();
};

export function registerTextDocument(context: vscode.ExtensionContext, bevyrlyIndex: BevyrlyIndex) {
    if (virtualTextProvider == null) {
        virtualTextProvider = createVirtualTextProvider(bevyrlyIndex);
        vscode.workspace.registerTextDocumentContentProvider("bevyrly", virtualTextProvider);
    }

    let disposableNewQuery = vscode.commands.registerCommand('bevyrly.newQuery', async () => {
        const result = await vscode.window.showInputBox({
            value: '',
            placeHolder: '&Transform *Vel E'
        });

        if (result) {
            showBevyrlyResultsAsTextDocument(result);
        }
    });
    context.subscriptions.push(disposableNewQuery);
}