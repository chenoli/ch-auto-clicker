import { useState } from "react";

import { useTranslation } from "react-i18next";

import { Badge } from "../../components/badge";
import { Button } from "../../components/button";
import { Callout } from "../../components/callout";

import { TitleBar } from "../main/title-bar";
import { SubScreenHeader } from "../shell/sub-screen-header";
import { MINIMAP, desktopBounds, segmentArrows, toMinimap } from "./geometry";

import type { OverlayMonitor, ScreenPoint } from "../../ipc";
import type { ReactElement } from "react";

export const DELAY_MAX = 60_000;
export const POINTS_MAX = 64;
const COORD_LIMIT = 100_000;

export interface PointsScreenProps {
  points: readonly ScreenPoint[];
  monitors: readonly OverlayMonitor[];
  layoutChanged: boolean;
  captureShortcut: string;
  errorCode?: string | null;
  onChange: (points: ScreenPoint[]) => void;
  onCapture: () => void;
  onAddManual: () => void;
  onBack: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

const two = (n: number) => String(n).padStart(2, "0");

export function parseCoordinate(raw: string): number | null {
  const n = Number(raw.trim());
  return raw.trim() !== "" && Number.isInteger(n) && Math.abs(n) <= COORD_LIMIT ? n : null;
}

export function parseDelay(raw: string): number | null {
  const text = raw.replace(/[+\s]|ms|—/g, "");
  if (text === "") return 0;
  const n = Number(text);
  return Number.isInteger(n) && n >= 0 ? Math.min(DELAY_MAX, n) : null;
}

export function reorder<T>(items: readonly T[], from: number, to: number): T[] {
  const next = [...items];
  const [moved] = next.splice(from, 1);
  if (moved !== undefined) next.splice(to, 0, moved);
  return next;
}

function CellInput({
  value,
  label,
  dashed = false,
  parse,
  onCommit,
}: {
  value: string;
  label: string;
  dashed?: boolean;
  parse: (raw: string) => number | null;
  onCommit: (n: number) => void;
}) {
  const [draft, setDraft] = useState(value);
  const commit = () => {
    const n = parse(draft);
    if (n === null) setDraft(value);
    else onCommit(n);
  };
  return (
    <input
      className={`cell-input ${dashed ? "cell-input--dashed" : ""}`}
      aria-label={label}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit();
      }}
    />
  );
}

export function PointsScreen({
  points,
  monitors,
  layoutChanged,
  captureShortcut,
  errorCode = null,
  onChange,
  onCapture,
  onAddManual,
  onBack,
  onMinimize,
  onClose,
}: PointsScreenProps): ReactElement {
  const { t } = useTranslation();
  const [dragging, setDragging] = useState<number | null>(null);
  const [over, setOver] = useState<number | null>(null);
  const [armed, setArmed] = useState<number | null>(null);

  const desktop = desktopBounds(monitors);
  const path = points.map((p) => toMinimap(desktop, p.x, p.y));
  const legend =
    monitors.length > 1
      ? t("points.manyScreens", { count: monitors.length, width: desktop.width, height: desktop.height })
      : t("points.oneScreen", { width: desktop.width, height: desktop.height });
  const update = (index: number, patch: Partial<ScreenPoint>) =>
    onChange(points.map((p, i) => (i === index ? { ...p, ...patch } : p)));

  return (
    <div className="flex h-full flex-col bg-bg">
      <TitleBar onMinimize={onMinimize} onClose={onClose} />
      <SubScreenHeader
        kicker={t("points.kicker")}
        title={t("points.title")}
        onBack={onBack}
        action={<Badge>{t("points.count", { count: points.length })}</Badge>}
      />
      <div className="flex-none px-4 pb-3">
        <div className="minimap" aria-hidden="true">
          <div className="minimap__grid" />
          {monitors.length > 1 &&
            monitors.map((m) => {
              const a = toMinimap(desktop, m.x, m.y);
              const b = toMinimap(desktop, m.x + m.width, m.y + m.height);
              return (
                <div
                  key={m.index}
                  className="minimap__monitor"
                  style={{ left: a.x, top: a.y, width: b.x - a.x, height: b.y - a.y }}
                />
              );
            })}
          <span className="minimap__legend">{legend}</span>
          <svg width={MINIMAP.width} height={MINIMAP.height} className="absolute inset-0">
            <polyline
              points={path.map((p) => `${p.x},${p.y}`).join(" ")}
              fill="none"
              stroke="var(--accent)"
              strokeWidth="1"
              strokeDasharray="3 3"
            />
          </svg>
          {segmentArrows(path).map((a, i) => (
            <span
              key={i}
              className="minimap__arrow"
              style={{ left: a.x, top: a.y, transform: `rotate(${a.angleDeg}deg)` }}
            />
          ))}
          {path.map((p, i) => (
            <span key={points[i]?.id ?? i} className="point-marker" style={{ left: p.x, top: p.y }}>
              {i + 1}
            </span>
          ))}
        </div>
      </div>
      {layoutChanged && (
        <div className="flex-none px-4 pb-3">
          <Callout>{t("points.layoutChanged")}</Callout>
        </div>
      )}
      {errorCode && (
        <div className="flex-none px-4 pb-3">
          <Callout tone="danger">
            {t(`errors.${errorCode}`, { defaultValue: t("errors.generic", { code: errorCode }) })}
          </Callout>
        </div>
      )}
      <div className="points-grid micro-label flex-none px-4 pb-1.5">
        <span />
        <span>{t("points.number")}</span>
        <span>{t("points.x")}</span>
        <span>{t("points.y")}</span>
        <span>{t("points.delay")}</span>
        <span />
      </div>
      <div className="min-h-0 flex-1 overflow-auto px-4">
        {points.length === 0 && (
          <p className="m-0 border-t border-divider py-4 text-12 text-fg-3">{t("points.empty")}</p>
        )}
        {points.map((p, i) => (
          <div
            key={p.id}
            className="points-grid point-row"
            draggable={armed === i}
            data-dragging={dragging === i}
            data-drop-target={over === i && dragging !== i}
            onDragStart={(e) => {
              e.dataTransfer.effectAllowed = "move";
              setDragging(i);
            }}
            onDragOver={(e) => {
              e.preventDefault();
              if (over !== i) setOver(i);
            }}
            onDrop={(e) => {
              e.preventDefault();
              if (dragging !== null && dragging !== i) onChange(reorder(points, dragging, i));
              setDragging(null);
              setOver(null);
            }}
            onDragEnd={() => {
              setDragging(null);
              setOver(null);
              setArmed(null);
            }}
          >
            <button
              type="button"
              className="drag-handle"
              aria-label={t("points.drag", { n: two(i + 1) })}
              onPointerDown={() => setArmed(i)}
              onPointerUp={() => setArmed(null)}
              onKeyDown={(e) => {
                const to = e.key === "ArrowUp" ? i - 1 : e.key === "ArrowDown" ? i + 1 : null;
                if (to === null || to < 0 || to >= points.length) return;
                e.preventDefault();
                onChange(reorder(points, i, to));
              }}
            >
              <span />
              <span />
              <span />
              <span />
              <span />
              <span />
            </button>
            <span className="font-display text-15 font-extrabold text-accent">{two(i + 1)}</span>
            <CellInput
              key={`x${p.x}`}
              label={`${t("points.x")} ${two(i + 1)}`}
              value={String(p.x)}
              parse={parseCoordinate}
              onCommit={(x) => update(i, { x })}
            />
            <CellInput
              key={`y${p.y}`}
              label={`${t("points.y")} ${two(i + 1)}`}
              value={String(p.y)}
              parse={parseCoordinate}
              onCommit={(y) => update(i, { y })}
            />
            <CellInput
              key={`d${p.delayMs}`}
              dashed
              label={`${t("points.delay")} ${two(i + 1)}`}
              value={p.delayMs ? `+${p.delayMs} ms` : t("points.noDelay")}
              parse={parseDelay}
              onCommit={(delayMs) => update(i, { delayMs })}
            />
            <button
              type="button"
              className="row-remove"
              aria-label={t("points.remove", { n: two(i + 1) })}
              onClick={() => onChange(points.filter((_, j) => j !== i))}
            >
              <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true">
                <path d="M1 1l8 8M9 1L1 9" stroke="currentColor" strokeWidth="1.3" fill="none" />
              </svg>
            </button>
          </div>
        ))}
      </div>
      <div className="flex flex-none gap-2 px-4 py-3">
        <button type="button" className="capture-button" onClick={onCapture}>
          {t("points.capture")}
          <span className="capture-button__keys">{captureShortcut}</span>
        </button>
        <Button className="h-11 px-3.5" onClick={onAddManual} disabled={points.length >= POINTS_MAX}>
          {t("points.manual")}
        </Button>
      </div>
      <div className="flex h-9 flex-none items-center border-t border-divider px-4 text-11 text-fg-3">
        {points.length > 0 && t("points.footer", { last: two(points.length) })}
      </div>
    </div>
  );
}
