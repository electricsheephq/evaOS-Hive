export const STARTER_PERSONA_PRESENTATION = [
  {
    avatarUrl: "/onboarding/starter-team/tars.webp",
    id: "builtin:fizz",
    name: "TARS",
  },
  {
    avatarUrl: "/onboarding/starter-team/samantha.webp",
    id: "builtin:honey",
    name: "Samantha",
  },
  {
    avatarUrl: "/onboarding/starter-team/hal-9000.webp",
    id: "builtin:bumble",
    name: "HAL 9000",
  },
] as const;

export const STARTER_PERSONA_NAMES = STARTER_PERSONA_PRESENTATION.map(
  ({ name }) => name,
);

export function starterPersonaAvatar(name: string): string | undefined {
  return STARTER_PERSONA_PRESENTATION.find((persona) => persona.name === name)
    ?.avatarUrl;
}
