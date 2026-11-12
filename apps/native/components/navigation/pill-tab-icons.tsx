import Svg, { Circle, Path, Rect } from "react-native-svg";

export type PillTabIconName = "dashboard" | "boarders" | "history";

const ICON_SIZE = 26;

const COLORS = {
  teal: "#3DDC97",
  blue: "#4C8DFF",
  pink: "#FF6B8A",
  amber: "#FFC24B",
  purple: "#8B5CF6",
  purpleSoft: "#A78BFA",
} as const;

function DashboardIcon({ muted }: { muted: boolean }) {
  const opacity = muted ? 0.82 : 1;

  return (
    <Svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 26 26" opacity={opacity}>
      <Rect x={4.5} y={12} width={3.6} height={9} rx={1.8} fill={COLORS.teal} />
      <Rect x={9.6} y={7} width={3.6} height={14} rx={1.8} fill={COLORS.blue} />
      <Rect x={14.7} y={3.5} width={3.6} height={17.5} rx={1.8} fill={COLORS.pink} />
      <Rect x={19.8} y={10} width={3.6} height={11} rx={1.8} fill={COLORS.amber} />
    </Svg>
  );
}

function BoardersIcon({ muted }: { muted: boolean }) {
  const opacity = muted ? 0.82 : 1;

  return (
    <Svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 26 26" opacity={opacity}>
      <Circle cx={13} cy={8.6} r={4.3} fill={COLORS.purpleSoft} />
      <Path
        d="M4.9 22.4c0-4.6 3.6-8 8.1-8s8.1 3.4 8.1 8c0 .6-.5 1.1-1.1 1.1H6c-.6 0-1.1-.5-1.1-1.1z"
        fill={COLORS.purple}
      />
    </Svg>
  );
}

function HistoryIcon({ muted }: { muted: boolean }) {
  const opacity = muted ? 0.82 : 1;

  return (
    <Svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 26 26" opacity={opacity}>
      <Circle
        cx={13}
        cy={13}
        r={8.9}
        stroke={COLORS.purple}
        strokeWidth={2.4}
        fill="none"
      />
      <Path
        d="M13 7.6V13l3.9 2.2"
        stroke={COLORS.purpleSoft}
        strokeWidth={2.2}
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
    </Svg>
  );
}

export function PillTabIcon({
  name,
  muted = false,
}: {
  name: PillTabIconName;
  muted?: boolean;
}) {
  if (name === "dashboard") return <DashboardIcon muted={muted} />;
  if (name === "boarders") return <BoardersIcon muted={muted} />;
  return <HistoryIcon muted={muted} />;
}
