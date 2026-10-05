import { useState } from "react";

import { useTranslation } from "react-i18next";

import { Button } from "../../components/button";
import { Callout } from "../../components/callout";

import { formatCps, formatOneDecimal } from "../../i18n/format";

import { TitleBar } from "../main/title-bar";
import { MAX_BLOCKS, clampBlock, moveBlock, rhythmTicks, variationStats } from "./variation-math";

import type { ReactElement } from "react";

export interface VariationEditorViewProps {
  initialBlocks: readonly number[];
  double: boolean;
  locale: string;
  minReliableMs: number;
  error?: string | null;
  busy?: boolean;
  onApply: (blocks: number[]) => void;
  onDisable: () => void;
  onClose: () => void;
  onMinimize: () => void;
}

const blockId = (i: number) => `B${String(i + 1).padStart(2, "0")}`;
const SCALE = [0, 100, 200, 300, 400, 500];

export function VariationEditorView({
  initialBlocks,
  double,
  locale,
  minReliableMs,
  error,
  busy = false,
  onApply,
  onDisable,
  onClose,
  onMinimize,
}: VariationEditorViewProps): ReactElement {
  const { t } = useTranslation();
  const [blocks, setBlocks] = useState<number[]>(() => (initialBlocks.length ? [...initialBlocks] : [100]));
  const [selected, setSelected] = useState(0);
  const [draft, setDraft] = useState(String(blocks[0] ?? 100));

  const select = (i: number) => {
    setSelected(i);
    setDraft(String(blocks[i] ?? ""));
  };
  const commitDraft = () => {
    const digits = draft.replace(/\D/g, "");
    if (!digits) {
      setDraft(String(blocks[selected] ?? ""));
      return;
    }
    const value = clampBlock(Number(digits));
    setBlocks((bs) => bs.map((b, i) => (i === selected ? value : b)));
    setDraft(String(value));
  };
  const move = (delta: -1 | 1) => {
    const [next, index] = moveBlock(blocks, selected, delta);
    setBlocks(next);
    setSelected(index);
  };
  const add = () => {
    if (blocks.length >= MAX_BLOCKS) return;
    const value = blocks[blocks.length - 1] ?? 100;
    setBlocks([...blocks, value]);
    setSelected(blocks.length);
    setDraft(String(value));
  };
  const remove = () => {
    if (blocks.length <= 1) return;
    const next = blocks.filter((_, i) => i !== selected);
    const index = Math.min(selected, next.length - 1);
    setBlocks(next);
    setSelected(index);
    setDraft(String(next[index]));
  };

  const stats = variationStats(blocks, double);
  const ticks = rhythmTicks(blocks, minReliableMs);
  const hasLow = blocks.some((b) => b < minReliableMs);

  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <div className="flex flex-none items-center gap-3 px-6 py-3.5">
        <button type="button" className="back-button" aria-label={t("variation.close")} onClick={onClose}>
          ‹
        </button>
        <div className="flex flex-1 flex-col gap-0.75">
          <span className="micro-label">{t("variation.kicker")}</span>
          <h1 className="m-0 font-display text-20 leading-none font-extrabold tracking-logo text-fg">
            {t("variation.title")}
          </h1>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-auto px-6 pt-1 pb-3">
        <div className="flex flex-col gap-2.5">
          <div className="flex justify-between">
            <span className="row-label">{t("variation.sequence")}</span>
            <span className="row-label tracking-status">{t("variation.widthHint")}</span>
          </div>
          <div className="flex h-21 items-stretch gap-1.5" role="listbox" aria-label={t("variation.sequence")}>
            {blocks.map((ms, i) => (
              <button
                key={i}
                type="button"
                role="option"
                aria-selected={i === selected}
                aria-pressed={i === selected}
                data-low={ms < minReliableMs}
                className="var-block"
                style={{ flex: `${Math.max(ms, 14)} 1 0` }}
                onClick={() => select(i)}
              >
                <span className="flex items-center justify-between">
                  <span className="var-block__id">{blockId(i)}</span>
                  {ms < minReliableMs && <span className="size-1.5 rounded-full bg-warn" />}
                </span>
                <span className="var-block__value">
                  {ms}
                  <span className="unit-suffix"> {t("units.ms")}</span>
                </span>
              </button>
            ))}
            <button
              type="button"
              className="var-add"
              aria-label={t("variation.add")}
              disabled={blocks.length >= MAX_BLOCKS}
              onClick={add}
            >
              +
            </button>
          </div>
          <div className="flex items-center gap-2 pr-12.5">
            <span className="cycle-line__edge" />
            <span className="cycle-line__rule" />
            <span className="text-10 font-semibold text-fg-3">{t("variation.back")}</span>
            <span className="cycle-line__rule" />
            <span className="cycle-line__edge" />
          </div>
        </div>

        <div className="flex items-center gap-2.5 border-y border-divider py-3">
          <span className="min-w-9 font-display text-15 font-extrabold text-accent">{blockId(selected)}</span>
          <label className="small-field">
            <input
              aria-label={`${t("variation.block")} ${blockId(selected)}`}
              inputMode="numeric"
              value={draft}
              onChange={(e) => setDraft(e.target.value.replace(/\D/g, ""))}
              onBlur={commitDraft}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitDraft();
              }}
            />
            <span className="unit-suffix">{t("units.ms")}</span>
          </label>
          <span className="flex-1" />
          <button type="button" className="chip-button" disabled={selected === 0} onClick={() => move(-1)}>
            {t("variation.moveLeft")}
          </button>
          <button
            type="button"
            className="chip-button"
            disabled={selected === blocks.length - 1}
            onClick={() => move(1)}
          >
            {t("variation.moveRight")}
          </button>
          <button
            type="button"
            className="chip-button chip-button--danger"
            disabled={blocks.length <= 1}
            onClick={remove}
          >
            {t("variation.remove")}
          </button>
        </div>

        <div className="flex flex-col gap-2.5">
          <div className="flex justify-between">
            <span className="row-label">{t("variation.rhythm")}</span>
            <span className="text-10 font-semibold text-fg-3">{t("variation.rhythmHint")}</span>
          </div>
          <div className="rhythm" aria-hidden="true">
            <div className="rhythm__grid">
              {SCALE.map((v) => (
                <span key={v} />
              ))}
            </div>
            <div className="rhythm__ticks">
              {ticks.map((tick, i) => (
                <span
                  key={i}
                  className="rhythm__tick"
                  data-start={tick.cycleStart}
                  data-low={tick.low}
                  style={{ left: `${tick.xPct}%` }}
                />
              ))}
            </div>
          </div>
          <div className="flex justify-between text-10 font-semibold text-fg-3 tabular-nums">
            {SCALE.map((v) => (
              <span key={v}>{v}</span>
            ))}
            <span>600 {t("units.ms")}</span>
          </div>
        </div>

        <div className="grid grid-cols-3 border-t border-divider">
          <div className="flex flex-col gap-1.5 py-3.5">
            <span className="micro-label">{t("variation.cycle")}</span>
            <span className="stat-value">
              {stats.cycleMs}
              <span className="unit-suffix"> {t("units.ms")}</span>
            </span>
          </div>
          <div className="flex flex-col gap-1.5 border-l border-divider py-3.5 pl-4">
            <span className="micro-label">{t("variation.average")}</span>
            <span className="stat-value">
              {formatOneDecimal(stats.averageMs, locale)}
              <span className="unit-suffix"> {t("units.ms")}</span>
            </span>
          </div>
          <div className="flex flex-col gap-1.5 border-l border-divider py-3.5 pl-4">
            <span className="micro-label">{t("variation.effective")}</span>
            <span className="stat-value text-accent">
              {formatCps(stats.cps, locale)}
              <span className="unit-suffix"> {t("units.cps")}</span>
            </span>
          </div>
        </div>

        {hasLow && (
          <Callout>
            {t("variation.warningPrefix")}{" "}
            <strong className="font-semibold text-warn">{t("variation.warningLimit", { limit: minReliableMs })}</strong>{" "}
            {t("variation.warningSuffix")}
          </Callout>
        )}
        {error && (
          <Callout tone="danger">
            {t(`errors.${error}`, { defaultValue: t("errors.generic", { code: error }) })}
          </Callout>
        )}
      </div>

      <div className="flex h-16 flex-none items-center justify-end gap-2 border-t border-divider px-6">
        <Button variant="text" className="h-9.5 px-3.5" disabled={busy} onClick={onDisable}>
          {t("variation.disable")}
        </Button>
        <Button variant="primary" className="h-9.5 px-4" disabled={busy} onClick={() => onApply(blocks)}>
          {t("variation.apply")}
        </Button>
      </div>
    </div>
  );
}
