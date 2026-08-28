import type { ModelRef, ModelSummary } from "@grove/runtime";
import { getSelectListTheme } from "@earendil-works/pi-coding-agent";
import {
  Container,
  type Focusable,
  Input,
  Key,
  matchesKey,
  SelectList,
  Spacer,
  Text,
} from "@earendil-works/pi-tui";

export class ModelPicker extends Container implements Focusable {
  readonly #search = new Input();
  readonly #list: SelectList;

  get focused(): boolean {
    return this.#search.focused;
  }

  set focused(value: boolean) {
    this.#search.focused = value;
  }

  constructor(
    availableModels: readonly ModelSummary[],
    current: ModelRef,
    onSelect: (model: ModelSummary) => void,
    onCancel: () => void,
  ) {
    super();
    const theme = getSelectListTheme();
    this.addChild(new Text(theme.selectedText(" Select model "), 1, 1));
    this.addChild(this.#search);
    this.addChild(new Spacer());

    const byValue = new Map(availableModels.map((model) => [selectValue(model), model]));
    this.#list = new SelectList(
      availableModels.map((model) => ({
        value: selectValue(model),
        label: model.name,
        description: `${model.ref.providerId} · ${formatTokens(model.contextWindow)} context`,
      })),
      Math.min(availableModels.length, 12),
      theme,
    );
    const currentIndex = availableModels.findIndex(
      (model) => modelValue(model.ref) === modelValue(current),
    );
    if (currentIndex >= 0) this.#list.setSelectedIndex(currentIndex);

    this.#list.onSelect = (item) => {
      const model = byValue.get(item.value);
      if (model) onSelect(model);
    };
    this.#list.onCancel = onCancel;
    this.#search.onSubmit = () => {
      const selected = this.#list.getSelectedItem();
      if (selected) this.#list.onSelect?.(selected);
    };
    this.#search.onEscape = onCancel;

    this.addChild(this.#list);
    this.addChild(
      new Text(
        theme.description(" Type to filter · ↑↓ navigate · Enter select · Esc cancel"),
        1,
        1,
      ),
    );
  }

  handleInput(data: string): void {
    if (
      matchesKey(data, Key.up) ||
      matchesKey(data, Key.down) ||
      matchesKey(data, Key.pageUp) ||
      matchesKey(data, Key.pageDown)
    ) {
      this.#list.handleInput(data);
      return;
    }
    this.#search.handleInput(data);
    this.#list.setFilter(this.#search.getValue());
  }
}

function modelValue(ref: ModelRef): string {
  return `${ref.agentId}:${ref.providerId}:${ref.modelId}`;
}

function selectValue(model: ModelSummary): string {
  return `${model.name}\u0000${modelValue(model.ref)}`;
}

function formatTokens(tokens: number): string {
  return tokens >= 1_000_000
    ? `${Math.round(tokens / 100_000) / 10}m`
    : `${Math.round(tokens / 1_000)}k`;
}
