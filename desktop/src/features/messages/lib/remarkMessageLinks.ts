/**
 * Remark plugin that detects bare product `…://message?…` URLs in text nodes and
 * replaces each with a custom `message-link` HAST element. Legacy
 * schemes stay plain text so native and managed protocol behavior cannot
 * bleed into one another. The `markdown.tsx` components map renders a match as
 * an inline pill (channel name + click-to-open) instead of the raw URL.
 *
 * Why this plugin exists: `remark-gfm`'s autolinker only covers `http(s)://`
 * and `www.`. Custom schemes like `buzz://` only reach the `<a>` component
 * override when the user wrote an explicit `[label](buzz://…)` link.
 *
 * Mirrors `remarkChannelLinks` / `remarkMentions` — same factory, same HAST
 * shape — so the rendering layer treats all three uniformly. Trailing
 * sentence punctuation (`. , ; : ! ?`) and unmatched closing brackets are
 * peeled off the match and emitted as plain text after the pill, so a URL
 * pasted at end-of-sentence still routes to the correct message id.
 */
// Explicit `.ts` extension lets this plugin be imported both by the Vite-built
// `markdown.tsx` and by `markdown.test.mjs` running under `node --test
// --experimental-strip-types`. `tsconfig.json` enables `allowImportingTsExtensions`.
import { createRemarkPrefixPlugin } from "../../../shared/lib/createRemarkPrefixPlugin.ts";
import { desktopProductPolicy } from "../../../shared/product/productIdentity.ts";

const TRAILING_PUNCTUATION_PATTERN = /[.,;:!?]+$/;

function activeMessageUrlPattern() {
  const scheme = desktopProductPolicy().deepLinkScheme;
  return new RegExp(`${scheme}:\\/\\/message\\?[^\\s<>"')\\]]+`, "g");
}

function trimMessageLinkMatch(matchText: string) {
  let value = matchText.replace(TRAILING_PUNCTUATION_PATTERN, "");
  while (/[)\]]$/.test(value) && isUnmatchedClosing(value)) {
    value = value.slice(0, -1).replace(TRAILING_PUNCTUATION_PATTERN, "");
  }
  return { value, trailing: matchText.slice(value.length) };
}

function isUnmatchedClosing(value: string): boolean {
  const closing = value[value.length - 1];
  const opening = closing === ")" ? "(" : "[";
  return value.split(closing).length > value.split(opening).length;
}

export default function remarkMessageLinks() {
  return createRemarkPrefixPlugin(activeMessageUrlPattern(), (matchText) => {
    const { value, trailing } = trimMessageLinkMatch(matchText);

    return {
      node: {
        type: "message-link",
        value,
        data: {
          hName: "message-link",
          hChildren: [{ type: "text", value }],
        },
      },
      trailing,
    };
  });
}
