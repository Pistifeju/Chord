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

export const SidePanelIcon = () => (
  <Icon>
    <rect x="1.75" y="2.75" width="12.5" height="10.5" rx="2.25" />
    <path d="M10 2.75v10.5" />
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

export const PlusIcon = () => (
  <Icon>
    <path d="M8 3v10M3 8h10" />
  </Icon>
);

export const ChatIcon = () => (
  <Icon>
    <path d="M3.5 3.25h9a1.75 1.75 0 0 1 1.75 1.75v4a1.75 1.75 0 0 1-1.75 1.75H8L5 13v-2.25H3.5A1.75 1.75 0 0 1 1.75 9v-4A1.75 1.75 0 0 1 3.5 3.25z" />
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

// Lucide's settings gear (ISC), drawn on its 24px grid and scaled to 16.
export const GearIcon = () => (
  <Icon>
    <g transform="scale(0.6667)">
      <path
        vectorEffect="non-scaling-stroke"
        d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
      />
      <circle vectorEffect="non-scaling-stroke" cx="12" cy="12" r="3" />
    </g>
  </Icon>
);

export const SlidersIcon = () => (
  <Icon>
    <path d="M2 5h5M11 5h3M2 11h2M8 11h6" />
    <circle cx="9" cy="5" r="1.75" />
    <circle cx="6" cy="11" r="1.75" />
  </Icon>
);

export const AppearanceIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="6" />
    <path d="M8 2a6 6 0 0 0 0 12z" fill="currentColor" />
  </Icon>
);

export const KeyboardIcon = () => (
  <Icon>
    <rect x="1.75" y="3.75" width="12.5" height="8.5" rx="1.75" />
    <path d="M4.5 6.5h.01M7 6.5h.01M9.5 6.5h.01M12 6.5h.01M5.5 9.5h5" />
  </Icon>
);

export const FolderIcon = ({ size = 24 }: { size?: number }) => (
  <Icon size={size}>
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

export const ImageIcon = ({ size }: { size?: number }) => (
  <Icon size={size}>
    <rect x="1.75" y="2.75" width="12.5" height="10.5" rx="1.75" />
    <circle cx="5.5" cy="6.25" r="1.25" />
    <path d="M14.25 10.5l-3.5-3.5-7.5 6.25" />
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

// Two paths, so each chevron can flip on its own.
export const CollapseAllIcon = () => (
  <Icon>
    <path className="collapse-chevron" d="M4.5 2.5L8 6l3.5-3.5" />
    <path className="collapse-chevron" d="M4.5 13.5L8 10l3.5 3.5" />
  </Icon>
);

export const ReloadIcon = () => (
  <Icon>
    <path d="M13 8a5 5 0 1 1-1.6-3.67M13 2.5v3h-3" />
  </Icon>
);

export const FileIcon = () => (
  <Icon>
    <path d="M4.5 1.75h4.75l3.25 3.25v8a1.25 1.25 0 0 1-1.25 1.25h-6.75A1.25 1.25 0 0 1 3.25 13V3A1.25 1.25 0 0 1 4.5 1.75z" />
    <path d="M9 1.75V5h3.5" />
  </Icon>
);

export const FilterIcon = () => (
  <Icon>
    <path d="M2.5 4.5h11M4.5 8h7M6.5 11.5h3" />
  </Icon>
);
