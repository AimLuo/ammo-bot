"use client";

import dynamic from "next/dynamic";
import { MapsSkeleton } from "./maps-content";

const MapsContent = dynamic(
  () => import("./maps-content").then((m) => ({ default: m.MapsContent })),
  { ssr: false, loading: () => <MapsSkeleton /> }
);

export default function MapsPage() {
  return (
    <div className="p-4 md:p-6">
      <MapsContent />
    </div>
  );
}
