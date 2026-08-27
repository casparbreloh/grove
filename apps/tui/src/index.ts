import { createHarness } from "@grove/runtime";
import {
  getMarkdownTheme,
  getSelectListTheme,
  initTheme,
  UserMessageComponent,
} from "@earendil-works/pi-coding-agent";
import {
  Container,
  Editor,
  Key,
  Loader,
  Markdown,
  matchesKey,
  ProcessTerminal,
  Spacer,
  TuiMainScreen,
} from "@earendil-works/pi-tui";

initTheme();

const harness = await createHarness();
const tui = new TuiMainScreen(new ProcessTerminal());
const transcript = new Container();
const status = new Container();
const selectListTheme = getSelectListTheme();
const markdownTheme = getMarkdownTheme();
const editor = new Editor(tui, {
  borderColor: selectListTheme.description,
  selectList: selectListTheme,
});
const loader = new Loader(
  tui,
  selectListTheme.selectedPrefix,
  selectListTheme.description,
  "Working...",
);

tui.addChild(transcript);
tui.addChild(status);
tui.addChild(editor);
tui.setFocus(editor);

let running = false;

editor.onSubmit = async (text) => {
  const prompt = text.trim();
  if (!prompt || running) return;

  running = true;
  editor.setText("");
  editor.disableSubmit = true;
  if (transcript.children.length) transcript.addChild(new Spacer());
  transcript.addChild(new UserMessageComponent(prompt, markdownTheme));
  transcript.addChild(new Spacer());
  const response = new Markdown("", 1, 0, markdownTheme);
  transcript.addChild(response);
  status.addChild(loader);
  loader.start();
  let output = "";
  tui.requestRender();

  try {
    await harness.prompt(prompt, (text) => {
      output += text;
      response.setText(output);
      tui.requestRender();
    });
  } catch (error) {
    response.setText(error instanceof Error ? error.message : "Request failed");
  } finally {
    running = false;
    editor.disableSubmit = false;
    loader.stop();
    status.clear();
    tui.setFocus(editor);
    tui.requestRender();
  }
};

tui.addInputListener((data) => {
  if (matchesKey(data, Key.escape) && running) {
    harness.abort();
    return { consume: true };
  }
  if (matchesKey(data, Key.ctrl("c"))) {
    if (running) harness.abort();
    else shutdown();
    return { consume: true };
  }
  return undefined;
});

tui.start();

function shutdown(): never {
  tui.stop();
  process.exit(0);
}
