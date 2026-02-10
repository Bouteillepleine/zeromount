import type { PackagesInfo } from 'kernelsu';
import { listPackages as ksuListPackages, getPackagesInfo as ksuGetPackagesInfo } from 'kernelsu';
export { spawn } from 'kernelsu';

interface KsuExecResult {
  errno: number;
  stdout: string;
  stderr: string;
}

let execCounter = 0;

const VALID_PACKAGE_PATTERN = /^[a-zA-Z][a-zA-Z0-9_.]*$/;

function isValidPackageName(name: string): boolean {
  return VALID_PACKAGE_PATTERN.test(name) && name.length <= 256;
}

export async function ksuExec(cmd: string, timeoutMs = 30000): Promise<KsuExecResult> {
  const ksu = globalThis.ksu;
  if (!ksu?.exec) {
    return { errno: -1, stdout: '', stderr: 'KSU not available' };
  }

  return new Promise((resolve) => {
    const callbackName = `ksu_api_cb_${Date.now()}_${execCounter++}` as const;

    const timeoutId = setTimeout(() => {
      delete window[callbackName];
      resolve({ errno: -1, stdout: '', stderr: 'timeout' });
    }, timeoutMs);

    window[callbackName] = (errno: number, stdout: string, stderr: string) => {
      clearTimeout(timeoutId);
      delete window[callbackName];
      resolve({ errno, stdout, stderr });
    };

    try {
      ksu.exec(cmd, '{}', callbackName);
    } catch {
      clearTimeout(timeoutId);
      delete window[callbackName];
      resolve({ errno: -1, stdout: '', stderr: 'exec failed' });
    }
  });
}

export async function listPackages(type: 'all' | 'user' | 'system'): Promise<string[]> {
  if (globalThis.ksu?.listPackages) {
    try {
      const result = ksuListPackages(type);
      if (Array.isArray(result) && result.length > 0) return result;
    } catch { /* fallback */ }
  }

  const pmFlags = { all: '', user: '-3', system: '-s' };
  const { stdout, errno } = await ksuExec(`pm list packages ${pmFlags[type]} | sed 's/package://'`);
  if (errno === 0 && stdout.trim()) {
    return stdout.trim().split('\n').filter(Boolean);
  }
  return [];
}

export async function getPackagesInfo(packageNames: string[]): Promise<PackagesInfo[]> {
  if (!packageNames.length) return [];

  if (globalThis.ksu?.getPackagesInfo) {
    try {
      const result = ksuGetPackagesInfo(packageNames);
      if (Array.isArray(result) && result.length > 0) return result;
    } catch { /* fallback */ }
  }

  const results: PackagesInfo[] = [];
  for (const packageName of packageNames) {
    if (!isValidPackageName(packageName)) {
      results.push({ packageName, appLabel: packageName, versionName: '', versionCode: 0, isSystem: false, uid: -1 });
      continue;
    }
    const { stdout, errno } = await ksuExec(
      `pm path ${packageName} 2>/dev/null | head -1 | sed 's/package://' | xargs -I{} aapt dump badging {} 2>/dev/null | grep "application-label:" | head -1 | sed "s/application-label:'\\(.*\\)'/\\1/"`
    );
    results.push({
      packageName,
      appLabel: errno === 0 && stdout.trim() ? stdout.trim() : packageName,
      versionName: '',
      versionCode: 0,
      isSystem: false,
      uid: -1,
    });
  }
  return results;
}

// Fetch label for a single app via aapt (used for newly installed apps)
export async function getAppLabelViaAapt(packageName: string): Promise<string | null> {
  if (!isValidPackageName(packageName)) return null;
  const { stdout, errno } = await ksuExec(
    `pm path ${packageName} 2>/dev/null | head -1 | sed 's/package://' | xargs -I{} aapt dump badging {} 2>/dev/null | grep "application-label:" | head -1 | sed "s/application-label:'\\(.*\\)'/\\1/"`
  );
  return errno === 0 && stdout.trim() ? stdout.trim() : null;
}
