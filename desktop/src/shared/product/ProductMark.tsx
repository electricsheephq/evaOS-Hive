import type { ComponentProps } from "react";

import { BuzzMark } from "@/shared/ui/buzz-logo/BuzzMark";
import { desktopProductPolicy } from "./productIdentity";

type ProductMarkProps = ComponentProps<"svg"> & {
  imageClassName?: string;
};

export function ProductMark({
  className,
  imageClassName,
  ...props
}: ProductMarkProps) {
  const policy = desktopProductPolicy();
  if (!policy.managed) {
    return <BuzzMark className={className} {...props} />;
  }
  return (
    <img
      alt="evaOS Teams"
      className={imageClassName ?? className}
      src="/evaos-teams-icon.png"
    />
  );
}
