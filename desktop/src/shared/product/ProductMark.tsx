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
  if (!desktopProductPolicy().managed) {
    return <BuzzMark className={className} {...props} />;
  }
  return (
    <img
      alt="Hive"
      className={imageClassName ?? className}
      src="/hive-icon.png"
    />
  );
}
