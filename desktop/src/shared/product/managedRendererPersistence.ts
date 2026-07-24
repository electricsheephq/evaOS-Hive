import { desktopProductPolicy } from "./productIdentity";

/**
 * Renderer-persisted records that can contain message bodies, attachment
 * metadata, or decrypted relay events. Managed builds keep these values in
 * memory only; the relay remains the durable message store.
 */
export const MANAGED_SENSITIVE_STORAGE_PREFIXES = Object.freeze([
  "buzz-drafts.v1",
  "buzz-drafts.v2",
  "buzz-channel-messages.v1",
]);

export function rendererContentPersistenceAllowed(): boolean {
  return !desktopProductPolicy().managed;
}

/**
 * Remove content-bearing renderer records before managed providers mount.
 *
 * This intentionally preserves ordinary device preferences and non-content
 * read markers. Sign-out still clears the whole managed WebKit origin.
 */
export function clearManagedSensitiveRendererState(
  storage: Storage = window.localStorage,
): number {
  if (rendererContentPersistenceAllowed()) {
    return 0;
  }

  const keys: string[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (
      key !== null &&
      MANAGED_SENSITIVE_STORAGE_PREFIXES.some((prefix) =>
        key.startsWith(prefix),
      )
    ) {
      keys.push(key);
    }
  }
  for (const key of keys) {
    storage.removeItem(key);
  }
  return keys.length;
}
