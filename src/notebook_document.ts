import * as vscode from 'vscode';
import { BevyrlyIndex, startBevyrlyIndexing } from '.';
import { expandLinkFromName, expandSystemFromName } from './extension';

interface RawNotebook {
    cells: RawNotebookCell[];
}

interface RawNotebookCell {
    source: string[];
}

class BevyrlyController implements vscode.Disposable {
    readonly controllerId = 'bevyrly-controller-id';
    readonly notebookType = 'bevyrly-notebook';
    readonly label = 'My Bevyrly Notebook';
    readonly supportedLanguages = ['rust'];

    private _bevyrlyIndex: BevyrlyIndex;
    private readonly _controller: vscode.NotebookController;
    private _executionOrder = 0;

    constructor(context: vscode.ExtensionContext, bevyrlyIndex: BevyrlyIndex) {
        this._bevyrlyIndex = bevyrlyIndex;
        if (!this._bevyrlyIndex.isInitialized) {
            startBevyrlyIndexing(context, bevyrlyIndex);
        }

        this._controller = vscode.notebooks.createNotebookController(
            this.controllerId,
            this.notebookType,
            this.label
        );

        this._controller.supportedLanguages = this.supportedLanguages;
        this._controller.supportsExecutionOrder = true;
        this._controller.executeHandler = this._execute.bind(this);
    }

    dispose() { }

    private _execute(
        cells: vscode.NotebookCell[],
        _notebook: vscode.NotebookDocument,
        _controller: vscode.NotebookController
    ): void {
        for (let cell of cells) {
            this._doExecution(cell);
        }
    }

    private async _doExecution(cell: vscode.NotebookCell): Promise<void> {
        const execution = this._controller.createNotebookCellExecution(cell);
        execution.executionOrder = ++this._executionOrder;
        execution.start(Date.now());

        let query = cell.document.getText();
        let result = [];

        for (const item of this._bevyrlyIndex.get(query)) {
            let expandedLink = expandLinkFromName(this._bevyrlyIndex, item);
            if (expandedLink) {
                let [loc, _] = expandedLink;
                const start = loc.src.l(loc[0]) + 1;
                const end = loc.src.l(loc[1]) + 1;
                const path = vscode.Uri.parse(loc.src.filepath?.replace("file:///", "") ?? "").path.split('/src/').pop();

                result.push(new vscode.NotebookCellOutput([
                    vscode.NotebookCellOutputItem.text("═══════════╣  <a style='color: #cccccc; text-decoration: none;' href='" +
                        (loc.src.filepath ?? "") + ":" + start + "'>Go to: <b>" + path + "</b>, lines <b>" +
                        start + "-" + end + "</b></a>  ╠══════════════════════════════════════════════════", 'text/html'),
                ]));

                let text = expandSystemFromName(this._bevyrlyIndex, item, false);
                let bodyStart = text.indexOf("{");
                text = text.slice(0, bodyStart) + "{ /* ... */ }";
                result.push(new vscode.NotebookCellOutput([
                    vscode.NotebookCellOutputItem.text(text, 'text/x-rust'),
                ]));
            }
        }

        execution.replaceOutput(result, cell);
        execution.end(true, Date.now());
    }
}

class BevyrlyNotebookSerializer implements vscode.NotebookSerializer {
    deserializeNotebook(content: Uint8Array, token: vscode.CancellationToken): vscode.NotebookData | Thenable<vscode.NotebookData> {
        var contents = new TextDecoder().decode(content);

        let raw: RawNotebookCell[];
        try {
            raw = (<RawNotebook>JSON.parse(contents)).cells;
        } catch {
            raw = [];
        }

        const cells = raw.map(
            item =>
                new vscode.NotebookCellData(
                    vscode.NotebookCellKind.Code,
                    item.source.join('\n'),
                    'markdown'
                )
        );

        return new vscode.NotebookData(cells);
    }

    async serializeNotebook(
        data: vscode.NotebookData,
        _token: vscode.CancellationToken
    ): Promise<Uint8Array> {
        let contents: RawNotebookCell[] = [];

        for (const cell of data.cells) {
            contents.push({
                source: cell.value.split(/\r?\n/g)
            });
        }

        return new TextEncoder().encode(JSON.stringify(contents));
    }
}

export function registerNotebookDocument(context: vscode.ExtensionContext, bevyrlyIndex: BevyrlyIndex) {
    context.subscriptions.push(
        vscode.workspace.registerNotebookSerializer('bevyrly-notebook', new BevyrlyNotebookSerializer())
    );
    context.subscriptions.push(new BevyrlyController(context, bevyrlyIndex));

    let disposableNewQuery = vscode.commands.registerCommand('bevyrly.newNotebook', async () => {
        var setting: vscode.Uri = vscode.Uri.parse("untitled:untitled.bevyrly");
        vscode.workspace.openNotebookDocument(setting).then((doc) => {
            vscode.window.showNotebookDocument(doc);
        }, (error: any) => {
            console.error(error);
            debugger;
        });
    });

    context.subscriptions.push(disposableNewQuery);
}