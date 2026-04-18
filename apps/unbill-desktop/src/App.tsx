// unbill desktop UI. Implementation begins at M5.
// See DESIGN.md for React architecture: TanStack Query + useUnbillEvent hook + Shadcn/Tailwind.

import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="flex h-10 shrink-0 items-center justify-between bg-neutral-900 px-3 select-none"
    >
      <span data-tauri-drag-region className="text-sm font-medium text-white/70">
        unbill
      </span>
      <div className="flex gap-2">
        <button
          onClick={() => win.minimize()}
          className="flex h-6 w-6 items-center justify-center rounded-full bg-yellow-400 hover:bg-yellow-300 transition-colors text-yellow-900 text-xs font-bold"
          aria-label="Minimize"
        >−</button>
        <button
          onClick={() => win.toggleMaximize()}
          className="flex h-6 w-6 items-center justify-center rounded-full bg-green-500 hover:bg-green-400 transition-colors text-green-900 text-xs font-bold"
          aria-label="Maximize"
        >+</button>
        <button
          onClick={() => win.close()}
          className="flex h-6 w-6 items-center justify-center rounded-full bg-red-500 hover:bg-red-400 transition-colors text-red-900 text-xs font-bold"
          aria-label="Close"
        >×</button>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-neutral-950 text-white">
      <TitleBar />
      <div className="flex flex-1 items-center justify-center">
        <p className="text-xl font-semibold">unbill — coming in M5</p>
      </div>
    </div>
  );
}
