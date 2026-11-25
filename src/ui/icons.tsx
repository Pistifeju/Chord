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

export const ChevronDownIcon = () => (
  <Icon size={12}>
    <path d="M4 6l4 4 4-4" />
  </Icon>
);

export const ArrowUpIcon = ({ size }: { size?: number }) => (
  <Icon size={size}>
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

function Logo({ side, children }: { side: number; children: ReactNode }) {
  const margin = side / 8;
  return (
    <svg width={16} height={16} viewBox={`${-margin} ${-margin} ${side + 2 * margin} ${side + 2 * margin}`} fillRule="evenodd" aria-hidden="true">
      {children}
    </svg>
  );
}

export const PiIcon = () => (
  <Logo side={560}>
    <path style={{ fill: "var(--brand-pi-coral)" }} d="M420 280H280V140H0V0H420V280Z" />
    <path style={{ fill: "var(--brand-pi-yellow)" }} d="M560 560H420V280H560V560Z" />
    <path style={{ fill: "var(--brand-pi-blue)" }} d="M140 560H0V140H140V280H280V420H140V560Z" />
  </Logo>
);

export const CodexIcon = () => (
  <Logo side={24}>
    <defs>
      <linearGradient id="codex-logo" x1="0" x2="0" y1="0" y2="1">
        <stop style={{ stopColor: "var(--brand-codex-top)" }} />
        <stop offset=".5" style={{ stopColor: "var(--brand-codex-middle)" }} />
        <stop offset="1" style={{ stopColor: "var(--brand-codex-bottom)" }} />
      </linearGradient>
    </defs>
    <path
      fill="url(#codex-logo)"
      d="M8.086.457a6.105 6.105 0 013.046-.415c1.333.153 2.521.72 3.564 1.7a.117.117 0 00.107.029c1.408-.346 2.762-.224 4.061.366l.063.03.154.076c1.357.703 2.33 1.77 2.918 3.198.278.679.418 1.388.421 2.126a5.655 5.655 0 01-.18 1.631.167.167 0 00.04.155 5.982 5.982 0 011.578 2.891c.385 1.901-.01 3.615-1.183 5.14l-.182.22a6.063 6.063 0 01-2.934 1.851.162.162 0 00-.108.102c-.255.736-.511 1.364-.987 1.992-1.199 1.582-2.962 2.462-4.948 2.451-1.583-.008-2.986-.587-4.21-1.736a.145.145 0 00-.14-.032c-.518.167-1.04.191-1.604.185a5.924 5.924 0 01-2.595-.622 6.058 6.058 0 01-2.146-1.781c-.203-.269-.404-.522-.551-.821a7.74 7.74 0 01-.495-1.283 6.11 6.11 0 01-.017-3.064.166.166 0 00.008-.074.115.115 0 00-.037-.064 5.958 5.958 0 01-1.38-2.202 5.196 5.196 0 01-.333-1.589 6.915 6.915 0 01.188-2.132c.45-1.484 1.309-2.648 2.577-3.493.282-.188.55-.334.802-.438.286-.12.573-.22.861-.304a.129.129 0 00.087-.087A6.016 6.016 0 015.635 2.31C6.315 1.464 7.132.846 8.086.457zm-.804 7.85a.848.848 0 00-1.473.842l1.694 2.965-1.688 2.848a.849.849 0 001.46.864l1.94-3.272a.849.849 0 00.007-.854l-1.94-3.393zm5.446 6.24a.849.849 0 000 1.695h4.848a.849.849 0 000-1.696h-4.848z"
    />
  </Logo>
);

export const ClaudeIcon = () => (
  <Logo side={24}>
    <path
      style={{ fill: "var(--brand-claude)" }}
      d="M4.709 15.955l4.72-2.647.08-.23-.08-.128H9.2l-.79-.048-2.698-.073-2.339-.097-2.266-.122-.571-.121L0 11.784l.055-.352.48-.321.686.06 1.52.103 2.278.158 1.652.097 2.449.255h.389l.055-.157-.134-.098-.103-.097-2.358-1.596-2.552-1.688-1.336-.972-.724-.491-.364-.462-.158-1.008.656-.722.881.06.225.061.893.686 1.908 1.476 2.491 1.833.365.304.145-.103.019-.073-.164-.274-1.355-2.446-1.446-2.49-.644-1.032-.17-.619a2.97 2.97 0 01-.104-.729L6.283.134 6.696 0l.996.134.42.364.62 1.414 1.002 2.229 1.555 3.03.456.898.243.832.091.255h.158V9.01l.128-1.706.237-2.095.23-2.695.08-.76.376-.91.747-.492.584.28.48.685-.067.444-.286 1.851-.559 2.903-.364 1.942h.212l.243-.242.985-1.306 1.652-2.064.73-.82.85-.904.547-.431h1.033l.76 1.129-.34 1.166-1.064 1.347-.881 1.142-1.264 1.7-.79 1.36.073.11.188-.02 2.856-.606 1.543-.28 1.841-.315.833.388.091.395-.328.807-1.969.486-2.309.462-3.439.813-.042.03.049.061 1.549.146.662.036h1.622l3.02.225.79.522.474.638-.079.485-1.215.62-1.64-.389-3.829-.91-1.312-.329h-.182v.11l1.093 1.068 2.006 1.81 2.509 2.33.127.578-.322.455-.34-.049-2.205-1.657-.851-.747-1.926-1.62h-.128v.17l.444.649 2.345 3.521.122 1.08-.17.353-.608.213-.668-.122-1.374-1.925-1.415-2.167-1.143-1.943-.14.08-.674 7.254-.316.37-.729.28-.607-.461-.322-.747.322-1.476.389-1.924.315-1.53.286-1.9.17-.632-.012-.042-.14.018-1.434 1.967-2.18 2.945-1.726 1.845-.414.164-.717-.37.067-.662.401-.589 2.388-3.036 1.44-1.882.93-1.086-.006-.158h-.055L4.132 18.56l-1.13.146-.487-.456.061-.746.231-.243 1.908-1.312-.006.006z"
    />
  </Logo>
);

export const OpenCodeIcon = () => (
  <Logo side={320}>
    <path style={{ fill: "var(--brand-opencode-fill)" }} d="M224 128V256H96V128H224Z" />
    <path style={{ fill: "var(--brand-opencode-frame)" }} d="M288 320H32V0H288V320ZM224 64H96V256H224V64Z" />
  </Logo>
);

export const CloseIcon = ({ size }: { size?: number }) => (
  <Icon size={size}>
    <path d="M4 4l8 8M12 4l-8 8" />
  </Icon>
);

export const MoreIcon = () => (
  <Icon>
    <path d="M3.5 8h0M8 8h0M12.5 8h0" strokeWidth="2" />
  </Icon>
);

export const MinusIcon = () => (
  <Icon>
    <path d="M3 8h10" />
  </Icon>
);

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

export const QuestionIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="6.25" />
    <path d="M6.25 6.25a1.75 1.75 0 1 1 2.6 1.53c-.5.28-.85.72-.85 1.22v.25M8 11.25h.01" />
  </Icon>
);

export const FolderIcon = ({ size = 24 }: { size?: number } = {}) => (
  <Icon size={size}>
    <path d="M1.75 4.25c0-.83.67-1.5 1.5-1.5h3l1.5 1.75h5c.83 0 1.5.67 1.5 1.5v6.25c0 .83-.67 1.5-1.5 1.5h-9.5c-.83 0-1.5-.67-1.5-1.5z" />
  </Icon>
);

export const WorkspacesIcon = () => (
  <Icon>
    <rect x="2" y="2.25" width="12" height="4.5" rx="1.5" />
    <rect x="2" y="9.25" width="12" height="4.5" rx="1.5" />
  </Icon>
);

export const SchemeIcon = () => (
  <Icon>
    <rect x="2.25" y="2.25" width="11.5" height="11.5" rx="3" />
    <path d="M6 10.75 8 5.25l2 5.5M6.6 9h2.8" />
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

export const CheckIcon = ({ size }: { size?: number }) => (
  <Icon size={size}>
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

export const BranchIcon = () => (
  <Icon>
    <circle cx="4.75" cy="3.5" r="1.5" />
    <circle cx="4.75" cy="12.5" r="1.5" />
    <circle cx="11.25" cy="5" r="1.5" />
    <path d="M4.75 5v6M11.25 6.5c0 2.5-6.5 1.5-6.5 4.5" />
  </Icon>
);

export const StashIcon = () => (
  <Icon>
    <rect x="2" y="3" width="12" height="3" rx="1" />
    <path d="M3 6v6.5a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V6M6.5 8.75h3" />
  </Icon>
);

export const DiffIcon = () => (
  <Icon>
    <path d="M4.5 1.75h4.75l3.25 3.25v8a1.25 1.25 0 0 1-1.25 1.25h-6.75A1.25 1.25 0 0 1 3.25 13V3A1.25 1.25 0 0 1 4.5 1.75z" />
    <path d="M8 5.5v4M6 7.5h4M6 11.5h4" />
  </Icon>
);

export const PlayIcon = () => (
  <Icon>
    <path d="M5 3.25l7 4.75-7 4.75z" fill="currentColor" />
  </Icon>
);

export const PhoneIcon = () => (
  <Icon>
    <rect x="4.5" y="1.75" width="7" height="12.5" rx="1.75" />
    <path d="M7 12h2" />
  </Icon>
);

export const EyeIcon = () => (
  <Icon>
    <path d="M1.75 8s2.25-4.25 6.25-4.25S14.25 8 14.25 8 12 12.25 8 12.25 1.75 8 1.75 8z" />
    <circle cx="8" cy="8" r="2" />
  </Icon>
);

export const StopIcon = () => (
  <Icon>
    <rect x="4" y="4" width="8" height="8" rx="1.5" fill="currentColor" />
  </Icon>
);

export const RecordIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="4.5" fill="currentColor" />
  </Icon>
);

export const HomeIcon = () => (
  <Icon>
    <path d="M2.5 7.25 8 2.75l5.5 4.5" />
    <path d="M4 6v7.25h8V6" />
  </Icon>
);

export const LockIcon = () => (
  <Icon>
    <rect x="3.25" y="7" width="9.5" height="7" rx="1.75" />
    <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
  </Icon>
);

export const UnlockIcon = () => (
  <Icon>
    <rect x="3.25" y="7" width="9.5" height="7" rx="1.75" />
    <path d="M5.5 7V5a2.5 2.5 0 0 1 4.9-.7" />
  </Icon>
);

export const ShakeIcon = () => (
  <Icon>
    <rect x="5.25" y="2.75" width="5.5" height="10.5" rx="1.5" />
    <path d="M2.75 6v4M13.25 6v4" />
  </Icon>
);

export const CameraIcon = () => (
  <Icon>
    <path d="M2.5 5.25h2.1l1-1.5h4.8l1 1.5h2.1v7.5h-11z" />
    <circle cx="8" cy="8.75" r="2.25" />
  </Icon>
);

export const GlobeIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="6.25" />
    <path d="M1.75 8h12.5M8 1.75c1.7 1.8 2.5 3.9 2.5 6.25S9.7 12.45 8 14.25C6.3 12.45 5.5 10.35 5.5 8S6.3 3.55 8 1.75z" />
  </Icon>
);

export const OpenOutsideIcon = () => (
  <Icon>
    <path d="M9.5 2.75h3.75V6.5M13.25 2.75L7.5 8.5M11.25 9.5v2.75a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1v-6.5a1 1 0 0 1 1-1H6.5" />
  </Icon>
);

export const ShieldIcon = () => (
  <Icon>
    <path d="M8 1.75 13 3.5v4.25c0 3-2.2 5.1-5 6.5-2.8-1.4-5-3.5-5-6.5V3.5z" />
  </Icon>
);

export const ShieldCheckIcon = () => (
  <Icon>
    <path d="M8 1.75 13 3.5v4.25c0 3-2.2 5.1-5 6.5-2.8-1.4-5-3.5-5-6.5V3.5z" />
    <path d="m5.75 7.75 1.5 1.5 3-3" />
  </Icon>
);

export const SearchIcon = () => (
  <Icon>
    <circle cx="7" cy="7" r="4.25" />
    <path d="m10.25 10.25 3.5 3.5" />
  </Icon>
);

export const PencilIcon = () => (
  <Icon>
    <path d="M10.75 2.5 13.5 5.25 6 12.75l-3.5.75.75-3.5z" />
    <path d="m9.25 4 2.75 2.75" />
  </Icon>
);

export const PlanIcon = () => (
  <Icon>
    <path d="M6.25 4h7M6.25 8h7M6.25 12h7" />
    <path d="M2.75 4h.01M2.75 8h.01M2.75 12h.01" strokeWidth="2" />
  </Icon>
);

export const BlockIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="5.75" />
    <path d="m3.95 3.95 8.1 8.1" />
  </Icon>
);
