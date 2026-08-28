import { randomUUID } from "node:crypto";

import {
  createGroveClient,
  type GroveProgress,
  type GroveUpdate,
  type ModelRef,
  type ModelSummary,
} from "@grove/runtime";
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
  type OverlayHandle,
  ProcessTerminal,
  Spacer,
  Text,
  TuiMainScreen,
} from "@earendil-works/pi-tui";

import { ModelPicker } from "./model-picker.ts";

initTheme();

const client = await createGroveClient();
const initial = await client.bootstrap();
const models = initial.models;
let session = initial.session;

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
  "Thinking…",
);
const footer = new Text();
const updates = client.watch({ after: initial.cursor })[Symbol.asyncIterator]();

let notice: string | undefined;
let shuttingDown = false;
let streamMessageId: string | undefined;
let streamReasoning = "";
let streamText = "";
let streamReasoningView: Text | undefined;
let streamTextView: Markdown | undefined;

tui.addChild(
  new Text(
    `${selectListTheme.selectedText(" Grove ")} ${selectListTheme.description("runtime harness")}`,
    0,
    1,
  ),
);
tui.addChild(transcript);
tui.addChild(status);
tui.addChild(editor);
tui.addChild(footer);
tui.setFocus(editor);

renderSession();
void watchUpdates();

editor.onSubmit = async (text) => {
  const prompt = text.trim();
  if (!prompt || session.phase.type === "running") return;
  if (!session.capabilities.prompt) {
    setNotice("This Agent does not support prompting");
    return;
  }

  editor.setText("");
  notice = undefined;
  const result = await client.execute({
    type: "session.prompt",
    commandId: randomUUID(),
    sessionId: session.id,
    text: prompt,
  });
  if (!result.ok) setNotice(result.error.message);
};

tui.addInputListener((data) => {
  if (
    matchesKey(data, Key.escape) &&
    session.phase.type === "running" &&
    session.capabilities.abort
  ) {
    void abortTurn();
    return { consume: true };
  }
  if (
    matchesKey(data, Key.ctrl("l")) &&
    session.phase.type === "idle" &&
    session.capabilities.selectModel &&
    !tui.hasOverlay()
  ) {
    showModelPicker();
    return { consume: true };
  }
  if (
    matchesKey(data, Key.ctrl("t")) &&
    session.phase.type === "idle" &&
    session.capabilities.setThinkingLevel &&
    !tui.hasOverlay()
  ) {
    void cycleThinkingLevel();
    return { consume: true };
  }
  if (matchesKey(data, Key.ctrl("c"))) {
    if (session.phase.type === "running" && session.capabilities.abort) void abortTurn();
    else shutdown();
    return { consume: true };
  }
  return undefined;
});

tui.start();

async function watchUpdates(): Promise<void> {
  try {
    while (true) {
      const update = await updates.next();
      if (update.done) return;
      applyUpdate(update.value);
    }
  } catch (error) {
    if (!shuttingDown) {
      setNotice(error instanceof Error ? error.message : "Runtime update stream failed");
    }
  }
}

function applyUpdate(update: GroveUpdate): void {
  if (update.kind === "event") {
    session = update.event.session;
    renderSession();
  } else {
    renderProgress(update.progress);
  }
  tui.requestRender();
}

function renderSession(): void {
  transcript.clear();
  resetStreamViews();

  for (const [index, message] of session.messages.entries()) {
    if (index > 0) transcript.addChild(new Spacer());
    if (message.role === "user") {
      const text = message.parts
        .filter((part) => part.type === "text")
        .map((part) => part.text)
        .join("\n");
      transcript.addChild(new UserMessageComponent(text, markdownTheme));
      continue;
    }

    for (const part of message.parts) {
      if (part.type === "text") transcript.addChild(new Markdown(part.text, 1, 0, markdownTheme));
      if (part.type === "reasoning" && part.text) {
        transcript.addChild(new Text(selectListTheme.description(part.text), 1, 0));
      }
      if (part.type === "tool-call") {
        transcript.addChild(new Text(selectListTheme.description(`↳ ${part.name}`), 1, 0));
      }
      if (part.type === "tool-result") {
        const marker = part.isError ? "✗" : "✓";
        transcript.addChild(new Text(selectListTheme.description(`${marker} ${part.name}`), 1, 0));
      }
    }
  }

  editor.disableSubmit = session.phase.type === "running";
  renderStatus();
  renderFooter();
}

function renderProgress(progress: GroveProgress): void {
  if (progress.messageId !== streamMessageId) {
    resetStreamViews();
    streamMessageId = progress.messageId;
    if (transcript.children.length > 0) transcript.addChild(new Spacer());
  }

  if (progress.type === "message.reasoning-delta") {
    streamReasoning += progress.delta;
    if (!streamReasoningView) {
      streamReasoningView = new Text("", 1, 0);
      transcript.addChild(streamReasoningView);
    }
    streamReasoningView.setText(selectListTheme.description(streamReasoning));
  }
  if (progress.type === "message.text-delta") {
    streamText += progress.delta;
    if (!streamTextView) {
      streamTextView = new Markdown("", 1, 0, markdownTheme);
      transcript.addChild(streamTextView);
    }
    streamTextView.setText(streamText);
  }
  if (progress.type === "tool.started") loader.setMessage(`Running ${progress.name}…`);
  if (progress.type === "tool.settled") loader.setMessage("Thinking…");
}

function resetStreamViews(): void {
  streamMessageId = undefined;
  streamReasoning = "";
  streamText = "";
  streamReasoningView = undefined;
  streamTextView = undefined;
}

function renderStatus(): void {
  loader.stop();
  status.clear();
  if (session.phase.type === "running") {
    loader.setMessage("Thinking…");
    status.addChild(loader);
    loader.start();
  } else if (notice) {
    status.addChild(new Text(selectListTheme.description(notice), 1, 0));
  } else if (session.lastTurn?.error) {
    status.addChild(new Text(selectListTheme.description(session.lastTurn.error.message), 1, 0));
  }
}

function renderFooter(): void {
  const current = findModel(session.model);
  const modelName = current?.name ?? session.model.modelId;
  const hints = [
    session.capabilities.selectModel ? "^L model" : undefined,
    session.capabilities.setThinkingLevel ? "^T thinking" : undefined,
    session.phase.type === "running" && session.capabilities.abort ? "Esc abort" : undefined,
  ].filter((hint): hint is string => Boolean(hint));
  footer.setText(
    selectListTheme.description(
      ` ${modelName} · ${session.thinkingLevel}${hints.length ? ` · ${hints.join(" · ")}` : ""}`,
    ),
  );
}

async function abortTurn(): Promise<void> {
  const result = await client.execute({
    type: "session.abort",
    commandId: randomUUID(),
    sessionId: session.id,
  });
  if (!result.ok) setNotice(result.error.message);
}

function showModelPicker(): void {
  let overlay: OverlayHandle | undefined;
  const close = () => {
    overlay?.hide();
    tui.setFocus(editor);
    tui.requestRender();
  };
  const picker = new ModelPicker(
    models,
    session.model,
    (model) => {
      close();
      void selectModel(model);
    },
    close,
  );
  overlay = tui.showOverlay(picker, { width: "70%", maxHeight: "70%", anchor: "center" });
  overlay.focus();
}

async function selectModel(model: ModelSummary): Promise<void> {
  notice = undefined;
  const result = await client.execute({
    type: "session.select-model",
    commandId: randomUUID(),
    sessionId: session.id,
    model: model.ref,
  });
  if (!result.ok) setNotice(result.error.message);
}

async function cycleThinkingLevel(): Promise<void> {
  notice = undefined;
  const levels = findModel(session.model)?.thinkingLevels ?? [];
  if (levels.length < 2) {
    setNotice("This model has no other thinking levels");
    return;
  }
  const currentIndex = levels.indexOf(session.thinkingLevel);
  const next = levels[(currentIndex + 1) % levels.length];
  if (!next) return;
  const result = await client.execute({
    type: "session.set-thinking-level",
    commandId: randomUUID(),
    sessionId: session.id,
    thinkingLevel: next,
  });
  if (!result.ok) setNotice(result.error.message);
}

function findModel(ref: ModelRef): ModelSummary | undefined {
  return models.find(
    (model) =>
      model.ref.agentId === ref.agentId &&
      model.ref.providerId === ref.providerId &&
      model.ref.modelId === ref.modelId,
  );
}

function setNotice(message: string): void {
  notice = message;
  renderStatus();
  tui.requestRender();
}

function shutdown(): never {
  shuttingDown = true;
  void updates.return?.();
  loader.stop();
  tui.stop();
  process.exit(0);
}
