import { NavLink, Outlet } from "react-router-dom";

const nav = [
  { to: "/", label: "Dashboard", icon: "◈" },
  { to: "/tasks", label: "Console", icon: "▶" },
  { to: "/settings", label: "Settings", icon: "⚙" },
];

export default function Layout() {
  return (
    <div className="flex min-h-screen">
      {/* Sidebar */}
      <aside className="fixed left-0 top-0 z-30 flex h-screen w-16 flex-col items-center gap-3 border-r border-white/[0.06] bg-slate-950/90 px-3 py-5 backdrop-blur-xl md:w-56 md:items-stretch">
        {/* Logo */}
        <div className="mb-6 flex items-center justify-center gap-3 md:px-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-cyan-400/10 text-base ring-1 ring-cyan-400/20">
            <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5" stroke="#22d3ee" strokeWidth="1.5">
              <ellipse cx="12" cy="10" rx="5" ry="3.5" fill="#22d3ee" fillOpacity="0.15" />
              <path d="M7 14q1 4 5 4 4 0 5-4" strokeLinecap="round" />
              <path d="M6 16q1 3 6 3 5 0 6-3" strokeLinecap="round" opacity="0.5" />
            </svg>
          </div>
          <span className="hidden text-sm font-semibold tracking-wider text-white md:block">MEMFLOW</span>
        </div>

        {/* Nav links */}
        {nav.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              `flex items-center justify-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all duration-200 md:justify-start ${
                isActive
                  ? "bg-cyan-400/10 text-cyan-200 shadow-[0_0_12px_rgba(34,211,238,0.08)]"
                  : "text-slate-500 hover:bg-white/[0.04] hover:text-slate-300"
              }`
            }
          >
            <span className="text-lg">{item.icon}</span>
            <span className="hidden md:block">{item.label}</span>
          </NavLink>
        ))}

        {/* Bottom status */}
        <div className="mt-auto flex justify-center md:px-3">
          <div className="flex items-center gap-2">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.5)]" />
            <span className="hidden text-[11px] uppercase tracking-wider text-slate-600 md:block">Online</span>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="ml-16 flex-1 md:ml-56">
        <div className="mx-auto max-w-5xl px-6 py-8 md:px-10 md:py-10">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
