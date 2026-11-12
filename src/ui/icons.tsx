// Line icons drawn in the current text color.
import type { ReactNode } from "react";

function Icon({ size = 16, children }: { size?: number; children: ReactNode }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {children}
    </svg>
  );
}

export const SidebarIcon = () => (
  <Icon>
    <rect x="1.75" y="2.75" width="12.5" height="10.5" rx="2.25" />
    <path d="M6 2.75v10.5" />
  </Icon>
);

export const ChevronIcon = () => (
  <Icon size={12}>
    <path d="M6 4l4 4-4 4" />
  </Icon>
);

export const ArrowUpIcon = () => (
  <Icon>
    <path d="M8 13V3M3.5 7.5L8 3l4.5 4.5" strokeWidth="1.8" />
  </Icon>
);

export const PlusIcon = () => (
  <Icon>
    <path d="M8 3v10M3 8h10" />
  </Icon>
);

export const CloseIcon = () => (
  <Icon>
    <path d="M4 4l8 8M12 4l-8 8" />
  </Icon>
);

export const MinusIcon = () => (
  <Icon>
    <path d="M3 8h10" />
  </Icon>
);

export const FolderIcon = () => (
  <Icon size={24}>
    <path d="M1.75 4.25c0-.83.67-1.5 1.5-1.5h3l1.5 1.75h5c.83 0 1.5.67 1.5 1.5v6.25c0 .83-.67 1.5-1.5 1.5h-9.5c-.83 0-1.5-.67-1.5-1.5z" />
  </Icon>
);

export const WarningIcon = () => (
  <Icon>
    <path d="M8 2.25l6.25 11H1.75z" />
    <path d="M8 6.5v3M8 11.25h.01" />
  </Icon>
);
