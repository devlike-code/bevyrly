import * as vscode from 'vscode';
import { BevyrlyIndex, startBevyrlyIndexing } from '.';
import { expandLinkFromName, expandSystemFromName } from './extension';

interface BevyrlyNotebook {
    cells: BevyrlyNotebookCell[];
}

interface BevyrlyNotebookCell {
    source: string[];
    kind: "markup" | "code";
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

        console.log("QUERY", query);
        if (query == "count") {
            console.log("!!!!!");
            let output = "<code>" + this._bevyrlyIndex.any + "</code> resources registered.";
            let result = new vscode.NotebookCellOutput([vscode.NotebookCellOutputItem.text(output, "text/html")]);
            execution.replaceOutput(result, cell);
            execution.end(true, Date.now());
            return;
        } else if (query.trim() == "?") {
            let output = [
                "<h1>Bevyrly</h1><hr />",
                "<b>Bevy</b> is <b>rly</b> useful, but requires some hygiene! Pronounced as /ˈbɛvə(ɹ)li/, derives from Old English, combining <i>befer</i> (\"beaver\") and <i>leah</i> (\"clearing\").<br />",
                "Bevyrly is a tool for quickly looking for Bevy systems by querying its arguments.<br />",
                "Bevyrly can be used as a notebook, enabling you to save useful queries and even document the many systems you use.",
                "<h2>Search control</h2><ul>",
                "<li><code>&Transform</code>: find all systems that include <code>Query&lt;&Transform&gt;</code> within it</li>",
                "<li><code>*Transform</code>: find all systems that include <code>Query&lt;&mut Transform&gt;</code> within it</li>",
                "<li><code>#Config</code>: find all systems that include <code>Res&lt;Config&gt;</code> within it</li>",
                "<li><code>$Config</code>: find all systems that include <code>ResMut&lt;Config&gt;</code> or <code>NonSendMut&lt;Config&gt;</code> within it</li>",
                "<li><code>&lt;ShipFireEvent</code>: find all systems that include <code>EventReader&lt;ShipFireEvent&gt;</code> within it</li>",
                "<li><code>&gt;ShipFireEvent</code>: find all systems that include <code>EventWriter&lt;ShipFireEvent&gt;</code> within it</li>",
                "<li><code>+Tag</code>: find all systems that include <code>With&lt;Tag&gt;</code> within it</li>",
                "<li><code>-Tag</code>: find all systems that include <code>Without&lt;Tag&gt;</code> within it</li>",
                "<li><code>JustText</code>: will match any of the above (might yield a <b>lot</b> of content)</li>",
                "</ul>",
                "<h2>Output control</h2><ul>",
                "<li><code>?</code>: prints this documentation</li>",
                "<li><code>my prompt goes here</code>: find and print locations of all systems that mention 'my', 'prompt', 'goes', and 'here'</li>",
                "<li><code>:my prompt goes here</code>: find and print declaration for all systems that mention 'my', 'prompt', 'goes', and 'here'</li>",
                "</ul>",
                "<h2>Examples</h2><ul>",
                "<li><code>:&Transform &gt;ShipFireEvent +Player</code>: prints full function declarations for any system that queries the <code>Transform</code> component immutably, accesses the <code>EventWriter&lt;ShipFireEvent&gt;</code>, and has a <code>With&lt;Player&gt;</code>.</li>",
                "<li><code>+Player -Player</code>: prints linkable locations to all the systems that require <code>With&lt;Player&gt;</code> and <code>Without&lt;Player&gt;</code> (possibly in different arguments)</li>",
                "<li><code>Foo Bar</code>: prints locations of all the systems that have the strings <code>Foo</code> and <code>Bar</code> <i>anywhere</i> in their arguments (including resources, components, etc.)</li>",
                "</ul>"
            ].join("<br>");

            let result = new vscode.NotebookCellOutput([vscode.NotebookCellOutputItem.text(output, "text/html")]);
            execution.replaceOutput(result, cell);
            execution.end(true, Date.now());
            return;
        }

        let result = [];

        let [response, long] = this._bevyrlyIndex.get(query);
        for (const item of response) {
            let expandedLink = expandLinkFromName(this._bevyrlyIndex, item);
            if (expandedLink) {
                let [loc, _] = expandedLink;
                const start = loc.src.l(loc[0]) + 1;
                const end = loc.src.l(loc[1]) + 1;
                const path = vscode.Uri.parse(loc.src.filepath?.replace("file:///", "") ?? "").path.split('/src/').pop();

                if (long == "long") {
                    result.push(new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.text("═══════════╣  <a style='color: #cccccc; text-decoration: none;' href='" +
                            (loc.src.filepath ?? "") + ":" + start + "'>Go to: <b>" + path + "</b>, lines <b>" +
                            start + "-" + end + "</b></a>  ╠═══════════", 'text/html'),
                    ]));

                    let text = expandSystemFromName(this._bevyrlyIndex, item, false);
                    let bodyStart = text.indexOf("{");
                    text = text.slice(0, bodyStart) + "{ /* ... */ }";
                    result.push(new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.text(text, 'text/x-rust'),
                    ]));
                } else {
                    result.push(new vscode.NotebookCellOutput([
                        vscode.NotebookCellOutputItem.text("<a style='color: #cccccc; text-decoration: none;' href='" +
                            (loc.src.filepath ?? "") + ":" + start + "'><b>[" + path + "] " + item + ":<b>" +
                            start + "</b></a>", 'text/html'),
                    ]));
                }
            }
        }

        execution.replaceOutput(result, cell);
        execution.end(true, Date.now());
    }
}

class BevyrlyNotebookSerializer implements vscode.NotebookSerializer {
    deserializeNotebook(content: Uint8Array, token: vscode.CancellationToken): vscode.NotebookData | Thenable<vscode.NotebookData> {
        if (content.length == 0) return new vscode.NotebookData([]);

        var contents = new TextDecoder().decode(content);

        let raw: BevyrlyNotebookCell[];
        try {
            raw = (<BevyrlyNotebook>JSON.parse(contents)).cells;
        } catch {
            raw = [];
        }

        const cells = raw.map(
            item =>
                new vscode.NotebookCellData(
                    item.kind == "code" ? vscode.NotebookCellKind.Code : vscode.NotebookCellKind.Markup,
                    item.source.join('\n'),
                    'rust'
                )
        );

        return new vscode.NotebookData(cells);
    }

    async serializeNotebook(
        data: vscode.NotebookData,
        _token: vscode.CancellationToken
    ): Promise<Uint8Array> {
        let contents: BevyrlyNotebookCell[] = [];

        for (const cell of data.cells) {
            if (cell.value.trim().length > 0) {
                contents.push({
                    source: cell.value.split(/\r?\n/g),
                    kind: cell.kind == vscode.NotebookCellKind.Code ? "code" : "markup",
                });
            }
        }

        return new TextEncoder().encode(JSON.stringify({ cells: contents }));
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