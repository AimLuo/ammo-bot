"use client";

import dynamic from "next/dynamic";

const DispatcherSection = dynamic(
  () =>
    import("../sections/dispatcher-section").then((m) => ({
      default: m.DispatcherSection,
    })),
  { ssr: false },
);

export default function DispatcherPage() {
  return <DispatcherSection />;
}
