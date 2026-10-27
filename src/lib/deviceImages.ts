import type { DeviceView } from "./types";
import alumixTkl from "../assets/devices/alumix-tkl.webp";
import ajazzAj159 from "../assets/devices/ajazz-aj159-nl.webp";
import ioGraphite from "../assets/devices/io-graphite-2-pro.webp";

const byRegistry: Record<string, string> = {
  "redsquare-alumix-tkl": alumixTkl,
  "ajazz-aj159-nl": ajazzAj159,
  "io-graphite-2-pro": ioGraphite,
};

const byVidPid: Record<string, string> = {
  "342d:e410": alumixTkl,
  "342d:e411": alumixTkl,
  "7768:6426": ajazzAj159,
  "7768:6d26": ajazzAj159,
  "4c4a:4155": ioGraphite,
};

export function deviceImage(device: Pick<DeviceView, "registryId" | "vid" | "pid">): string | null {
  if (device.registryId && byRegistry[device.registryId]) return byRegistry[device.registryId];
  if (device.vid != null && device.pid != null) {
    const key = `${device.vid.toString(16)}:${device.pid.toString(16)}`;
    return byVidPid[key] ?? null;
  }
  return null;
}
