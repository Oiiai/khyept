const vscode = require('vscode');

async function compileAndRun() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'khyept') {
    vscode.window.showErrorMessage('Open a Khyept source file before compiling.');
    return;
  }

  if (editor.document.isUntitled) {
    vscode.window.showErrorMessage('Save the Khyept source file before compiling.');
    return;
  }

  if (editor.document.isDirty && !(await editor.document.save())) {
    return;
  }

  const workspaceFolder = vscode.workspace.getWorkspaceFolder(editor.document);
  const task = new vscode.Task(
    { type: 'khyept', task: 'compile-run' },
    workspaceFolder || vscode.TaskScope.Global,
    'Khyept: Compile and Run',
    'Khyept',
    new vscode.ShellExecution('kyptc "${file}"', {
      cwd: '${fileDirname}'
    })
  );

  task.group = vscode.TaskGroup.Build;
  task.presentationOptions = {
    reveal: vscode.TaskRevealKind.Always,
    panel: vscode.TaskPanelKind.Shared,
    clear: true,
    focus: true,
    echo: true,
    showReuseMessage: true
  };

  await vscode.tasks.executeTask(task);
}

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand('khyept.build', compileAndRun)
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
