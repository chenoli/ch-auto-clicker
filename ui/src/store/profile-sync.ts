import { commands } from "../ipc";

import { replaceProfile, setCommandError, useAppStore } from "./app-store";

import type { Profile, ProfilesChanged } from "../ipc";

/** Last version Rust confirmed for each profile; every save is based on it. */
const serverVersion = new Map<string, string>();
/** Newest local edit not yet sent. */
const queued = new Map<string, Profile>();
const inflight = new Set<string>();

export function rememberServerVersions(profiles: readonly Profile[]): void {
  for (const p of profiles) serverVersion.set(p.id, p.updatedAt);
}

export function hasPendingEdits(id: string): boolean {
  return queued.has(id) || inflight.has(id);
}

async function flush(id: string): Promise<void> {
  const next = queued.get(id);
  if (!next) return;
  queued.delete(id);
  inflight.add(id);
  const result = await commands.profileUpdate(next, serverVersion.get(id) ?? next.updatedAt);
  inflight.delete(id);
  if (result.status === "ok") {
    serverVersion.set(id, result.data.updatedAt);
    if (queued.has(id)) {
      await flush(id);
    } else {
      replaceProfile(result.data);
    }
    return;
  }
  // The edit was refused (another window saved first, or invalid): drop newer local edits too and
  // show the latest confirmed version, which the ProfilesChanged event already delivered.
  queued.delete(id);
  setCommandError(result.error.code);
  const confirmed = lastConfirmed.get(id);
  if (confirmed) replaceProfile(confirmed);
}

/** Server copies from the last ProfilesChanged, to roll back a refused edit. */
const lastConfirmed = new Map<string, Profile>();

/** Applies an edit locally right away and saves it in the background, one request at a time. */
export function editProfile(id: string, mutate: (p: Profile) => Profile): void {
  const current = useAppStore.getState().profiles.find((p) => p.id === id);
  if (!current) return;
  const next = mutate(current);
  replaceProfile(next);
  setCommandError(null);
  queued.set(id, next);
  if (!inflight.has(id)) void flush(id);
}

/** Merges an event from Rust: profiles with local edits in flight keep the local copy. */
export function applyProfilesChanged(event: ProfilesChanged): void {
  for (const p of event.profiles) lastConfirmed.set(p.id, p);
  rememberServerVersions(event.profiles);
  useAppStore.setState((s) => {
    const local = new Map(s.profiles.map((p) => [p.id, p]));
    const profiles = event.profiles.map((p) => (hasPendingEdits(p.id) ? (local.get(p.id) ?? p) : p));
    return {
      profiles,
      settings: s.settings ? { ...s.settings, activeProfileId: event.activeProfileId } : s.settings,
    };
  });
}

/** Test helper: forget all sync state. */
export function resetProfileSync(): void {
  serverVersion.clear();
  queued.clear();
  inflight.clear();
  lastConfirmed.clear();
}
