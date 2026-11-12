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

export const ArrowDownIcon = () => (
  <Icon>
    <path d="M8 3v10M3.5 8.5L8 13l4.5-4.5" strokeWidth="1.5" />
  </Icon>
);

export const SparkleIcon = () => (
  <Icon>
    <path d="M8 1.5C8.6 5.4 10.6 7.4 14.5 8 10.6 8.6 8.6 10.6 8 14.5 7.4 10.6 5.4 8.6 1.5 8 5.4 7.4 7.4 5.4 8 1.5Z" fill="currentColor" stroke="none" />
  </Icon>
);

export const PlusIcon = () => (
  <Icon>
    <path d="M8 3v10M3 8h10" />
  </Icon>
);

export const ChatIcon = () => (
  <Icon>
    <path d="M3.5 2.75h9a1.75 1.75 0 0 1 1.75 1.75v5a1.75 1.75 0 0 1-1.75 1.75H8L5 13.5v-2.25H3.5A1.75 1.75 0 0 1 1.75 9.5v-5A1.75 1.75 0 0 1 3.5 2.75z" />
  </Icon>
);

export const CloseIcon = ({ size }: { size?: number }) => (
  <Icon size={size}>
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

export const CopyIcon = () => (
  <Icon>
    <rect x="5.75" y="5.75" width="8" height="8" rx="1.75" />
    <path d="M10.25 3.5V3.25A1.5 1.5 0 0 0 8.75 1.75h-5a1.5 1.5 0 0 0-1.5 1.5v5a1.5 1.5 0 0 0 1.5 1.5H4" />
  </Icon>
);

export const CheckIcon = () => (
  <Icon>
    <path d="M3 8.5l3.5 3.5L13 4.5" />
  </Icon>
);

export const ArrowLeftIcon = () => (
  <Icon>
    <path d="M13 8H3M7.5 3.5L3 8l4.5 4.5" strokeWidth="1.5" />
  </Icon>
);

export const ArrowRightIcon = () => (
  <Icon>
    <path d="M3 8h10M8.5 3.5L13 8l-4.5 4.5" strokeWidth="1.5" />
  </Icon>
);

export const PaperclipIcon = () => (
  <Icon>
    <path d="M13.25 7.5l-4.9 4.9a3 3 0 0 1-4.25-4.25l5.1-5.1a2 2 0 0 1 2.85 2.85l-5.1 5.1a1 1 0 0 1-1.4-1.4L10.4 4.7" />
  </Icon>
);

export const SkillIcon = () => (
  <Icon>
    <path d="M8 1.75C8.6 5.4 10.6 7.4 14.25 8 10.6 8.6 8.6 10.6 8 14.25 7.4 10.6 5.4 8.6 1.75 8 5.4 7.4 7.4 5.4 8 1.75Z" />
  </Icon>
);

export const PromptIcon = () => (
  <Icon>
    <path d="M4.5 1.75h4.75l3.25 3.25v8a1.25 1.25 0 0 1-1.25 1.25h-6.75A1.25 1.25 0 0 1 3.25 13V3A1.25 1.25 0 0 1 4.5 1.75z" />
    <path d="M9 1.75V5h3.5M5.5 8.25h5M5.5 11h3.5" />
  </Icon>
);

export const ExtensionIcon = () => (
  <Icon>
    <path d="M8 1.75l5.5 3v6.5l-5.5 3-5.5-3v-6.5z" />
    <path d="M2.5 4.75L8 7.75l5.5-3M8 7.75v6.5" />
  </Icon>
);
