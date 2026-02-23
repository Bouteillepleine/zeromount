package com.test.statcheck;

import android.app.Activity;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.os.Build;
import android.system.Os;
import android.system.OsConstants;
import android.system.StructStat;
import android.widget.TextView;

import android.os.Environment;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.ArrayList;
import java.util.List;

public class MainActivity extends Activity {

    private static final String ANDROID_DATA = "/sdcard/Android/data";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        new Thread(() -> {
            StringBuilder sb = new StringBuilder();
            sb.append("===== StatCheck v3.0 =====\n");
            sb.append("PID: ").append(android.os.Process.myPid()).append("\n");
            sb.append("UID: ").append(android.os.Process.myUid()).append("\n");
            sb.append("SDK: ").append(Build.VERSION.SDK_INT).append("\n");
            sb.append("ExtStorageMgr: ").append(
                Build.VERSION.SDK_INT >= 30 ? Environment.isExternalStorageManager() : "N/A"
            ).append("\n\n");

            int mountHits = grepFile("/proc/self/mountinfo", "overlay").size();
            sb.append("[1-6] Overlays: ").append(mountHits == 0 ? "CLEAN" : mountHits + " visible").append("\n");
            sb.append("[7] /data/adb: ").append(new File("/data/adb").exists() ? "DETECTED" : "CLEAN").append("\n");
            sb.append("[9] debuggable=").append(getProp("ro.debuggable"));
            sb.append(" type=").append(getProp("ro.build.type"));
            sb.append(" vbmeta=").append(getProp("ro.boot.vbmeta.device_state")).append("\n\n");

            testFileListApi(sb);
            testDirectStat(sb);
            testReaddirConsistency(sb);
            testSusfsVisibility(sb);

            sb.append("==========================================\n");

            String result = sb.toString();
            writeFile("/sdcard/statcheck.txt", result);
            writeFile(getFilesDir() + "/statcheck.txt", result);
            runOnUiThread(() -> ((TextView) findViewById(R.id.results)).setText(result));
        }).start();
    }

    private void testFileListApi(StringBuilder sb) {
        sb.append("[A] File.list() on ").append(ANDROID_DATA).append("\n");
        File dir = new File(ANDROID_DATA);
        sb.append("    isDirectory: ").append(dir.isDirectory()).append("\n");
        sb.append("    canRead:     ").append(dir.canRead()).append("\n");

        long start = System.nanoTime();
        String[] entries = dir.list();
        long elapsed = System.nanoTime() - start;

        if (entries == null) {
            sb.append("    list():      null (").append(elapsed / 1_000).append(" us)\n");
            sb.append("    >> FUSE blocks readdir for this UID on API ").append(Build.VERSION.SDK_INT).append("\n");
        } else {
            sb.append("    list():      ").append(entries.length).append(" entries (")
              .append(elapsed / 1_000).append(" us)\n");
            for (String e : entries) sb.append("      ").append(e).append("\n");
        }

        start = System.nanoTime();
        File[] files = dir.listFiles();
        elapsed = System.nanoTime() - start;
        sb.append("    listFiles(): ").append(files == null ? "null" : files.length + " files")
          .append(" (").append(elapsed / 1_000).append(" us)\n");

        sb.append("\n");
    }

    private void testDirectStat(StringBuilder sb) {
        List<String> allPkgs = getAllPackages();
        sb.append("[B] Direct Os.stat() on ").append(allPkgs.size()).append(" installed packages\n");

        try {
            StructStat parentSt = Os.stat(ANDROID_DATA);
            sb.append("    parent ino=").append(parentSt.st_ino)
              .append(" dev=0x").append(Long.toHexString(parentSt.st_dev))
              .append(" nlink=").append(parentSt.st_nlink).append("\n");
        } catch (Exception e) {
            sb.append("    parent stat FAILED: ").append(e.getMessage()).append("\n");
        }

        int okCount = 0, enoentCount = 0, eaccesCount = 0, otherCount = 0;
        long totalNs = 0;
        long maxNs = 0;
        String maxPkg = "";

        for (String pkg : allPkgs) {
            String path = ANDROID_DATA + "/" + pkg;
            long start = System.nanoTime();
            try {
                StructStat st = Os.stat(path);
                long elapsed = System.nanoTime() - start;
                totalNs += elapsed;
                if (elapsed > maxNs) { maxNs = elapsed; maxPkg = pkg; }
                sb.append("    OK   ").append(String.format("%7d", elapsed / 1000)).append(" us  ")
                  .append("ino=").append(String.format("%-8d", st.st_ino)).append(pkg).append("\n");
                okCount++;
            } catch (android.system.ErrnoException e) {
                long elapsed = System.nanoTime() - start;
                totalNs += elapsed;
                String err;
                if (e.errno == OsConstants.ENOENT) { err = "ENOENT"; enoentCount++; }
                else if (e.errno == OsConstants.EACCES) { err = "EACCES"; eaccesCount++; }
                else { err = "errno=" + e.errno; otherCount++; }
                sb.append("    FAIL ").append(String.format("%7d", elapsed / 1000)).append(" us  ")
                  .append(String.format("%-8s", err)).append(pkg).append("\n");
            } catch (Exception e) {
                sb.append("    ERR  ").append(pkg).append(": ").append(e.getMessage()).append("\n");
                otherCount++;
            }
        }

        sb.append("    ---\n");
        sb.append("    total: ").append(totalNs / 1_000_000).append(" ms | ")
          .append(okCount).append(" ok, ").append(enoentCount).append(" ENOENT, ")
          .append(eaccesCount).append(" EACCES, ").append(otherCount).append(" other\n");
        int count = allPkgs.size();
        sb.append("    avg: ").append(count > 0 ? totalNs / count / 1_000 : 0)
          .append(" us | max: ").append(maxNs / 1_000).append(" us (").append(maxPkg).append(")\n");

        if (enoentCount > 0 && okCount > 0) {
            sb.append("    >> MISMATCH: some ENOENT, some OK — partial SUSFS hiding\n");
        } else if (enoentCount > 0 && okCount == 0) {
            sb.append("    >> ALL ENOENT — SUSFS hiding stat for this UID\n");
        } else if (okCount > 0 && enoentCount == 0) {
            sb.append("    >> ALL accessible — SUSFS NOT hiding stat\n");
        }

        sb.append("\n");
    }

    // File.list() based — no subprocesses
    private void testReaddirConsistency(StringBuilder sb) {
        sb.append("[C] Readdir consistency (10 rounds, File.list)\n");

        int[] counts = new int[10];
        long[] times = new long[10];

        for (int i = 0; i < 10; i++) {
            long start = System.nanoTime();
            String[] entries = new File(ANDROID_DATA).list();
            times[i] = System.nanoTime() - start;
            counts[i] = entries != null ? entries.length : -1;
            sb.append("    round ").append(i + 1).append(": ")
              .append(counts[i]).append(" entries (")
              .append(times[i] / 1_000).append(" us)\n");
        }

        boolean consistent = true;
        for (int i = 1; i < 10; i++) {
            if (counts[i] != counts[0]) { consistent = false; break; }
        }

        if (consistent) {
            sb.append("    CONSISTENT: all rounds = ").append(counts[0]).append("\n");
        } else {
            sb.append("    !! FLICKER — count changes between calls !!\n");
        }

        sb.append("\n");
    }

    private void testSusfsVisibility(StringBuilder sb) {
        sb.append("[D] SUSFS visibility analysis\n");

        List<String> thirdParty = getAllPackages();
        sb.append("    installed packages: ").append(thirdParty.size()).append("\n");

        // Stat-only visibility — no readdir needed
        int hidden = 0, visible = 0;
        List<String> hiddenPkgs = new ArrayList<>();
        List<String> visiblePkgs = new ArrayList<>();

        for (String pkg : thirdParty) {
            try {
                Os.stat(ANDROID_DATA + "/" + pkg);
                visible++;
                visiblePkgs.add(pkg);
            } catch (Exception e) {
                hidden++;
                hiddenPkgs.add(pkg);
            }
        }

        sb.append("    stat OK (visible): ").append(visible).append("\n");
        sb.append("    stat ENOENT (hidden): ").append(hidden).append("\n");

        if (!hiddenPkgs.isEmpty()) {
            sb.append("    hidden:\n");
            for (String p : hiddenPkgs) sb.append("      - ").append(p).append("\n");
        }
        if (!visiblePkgs.isEmpty()) {
            sb.append("    visible:\n");
            for (String p : visiblePkgs) sb.append("      + ").append(p).append("\n");
        }

        sb.append("\n");
    }

    private List<String> getAllPackages() {
        List<String> pkgs = new ArrayList<>();
        try {
            for (ApplicationInfo ai : getPackageManager().getInstalledApplications(0)) {
                pkgs.add(ai.packageName);
            }
        } catch (Exception ignored) {}
        return pkgs;
    }

    private List<String> grepFile(String path, String keyword) {
        List<String> matches = new ArrayList<>();
        try (BufferedReader br = new BufferedReader(new FileReader(path))) {
            String line;
            while ((line = br.readLine()) != null) {
                if (line.contains(keyword)) matches.add(line);
            }
        } catch (Exception ignored) {}
        return matches;
    }

    private String getProp(String prop) {
        try {
            return (String) Class.forName("android.os.SystemProperties")
                .getMethod("get", String.class)
                .invoke(null, prop);
        } catch (Exception e) {
            return "?";
        }
    }

    private void writeFile(String path, String content) {
        try {
            java.io.FileWriter fw = new java.io.FileWriter(path);
            fw.write(content);
            fw.close();
        } catch (Exception ignored) {}
    }
}
