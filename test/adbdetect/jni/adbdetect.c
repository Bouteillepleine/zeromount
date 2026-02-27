#include <jni.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <dirent.h>
#include <sys/stat.h>
#include <android/log.h>

// Bionic symbol — not in NDK headers but always present on Android
int __system_property_get(const char *name, char *value);

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_nativeGetProperty(
        JNIEnv *env, jclass clazz, jstring key) {
    const char *k = (*env)->GetStringUTFChars(env, key, NULL);
    char value[92] = {};
    __system_property_get(k, value);
    (*env)->ReleaseStringUTFChars(env, key, k);
    return (*env)->NewStringUTF(env, value);
}

static jstring check_tcp_file(JNIEnv *env, const char *path, int port) {
    char target_hex[9];
    snprintf(target_hex, sizeof(target_hex), ":%04X", port);
    FILE *f = fopen(path, "r");
    if (!f) return (*env)->NewStringUTF(env, "ERROR:cannot_open");
    char line[512];
    fgets(line, sizeof(line), f);
    while (fgets(line, sizeof(line), f)) {
        if (strstr(line, target_hex)) {
            fclose(f);
            char result[64];
            char *tok = strtok(line, " \t");
            int field = 0;
            while (tok) {
                if (field == 1) {
                    snprintf(result, sizeof(result), "FOUND:%s", tok);
                    return (*env)->NewStringUTF(env, result);
                }
                tok = strtok(NULL, " \t");
                field++;
            }
            return (*env)->NewStringUTF(env, "FOUND");
        }
    }
    fclose(f);
    return (*env)->NewStringUTF(env, "NOT_FOUND");
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_readTcpEntry(
        JNIEnv *env, jclass clazz, jint port) {
    return check_tcp_file(env, "/proc/net/tcp", (int)port);
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_readTcp6Entry(
        JNIEnv *env, jclass clazz, jint port) {
    return check_tcp_file(env, "/proc/net/tcp6", (int)port);
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_readUnixAdbd(
        JNIEnv *env, jclass clazz) {
    FILE *f = fopen("/proc/net/unix", "r");
    if (!f) return (*env)->NewStringUTF(env, "ERROR:cannot_open");
    char line[512];
    fgets(line, sizeof(line), f);
    while (fgets(line, sizeof(line), f)) {
        if (strstr(line, "adbd") || strstr(line, "jdwp")) {
            fclose(f);
            line[strcspn(line, "\n")] = 0;
            size_t len = strlen(line);
            char result[256];
            snprintf(result, sizeof(result), "FOUND:%s",
                     len > 60 ? line + len - 60 : line);
            return (*env)->NewStringUTF(env, result);
        }
    }
    fclose(f);
    return (*env)->NewStringUTF(env, "NOT_FOUND");
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_findAdbdProc(
        JNIEnv *env, jclass clazz) {
    DIR *proc = opendir("/proc");
    if (!proc) return (*env)->NewStringUTF(env, "ERROR:cannot_open_proc");
    struct dirent *entry;
    char cmdline_path[64], cmdline[256];
    while ((entry = readdir(proc)) != NULL) {
        int is_pid = 1;
        for (int i = 0; entry->d_name[i]; i++) {
            if (entry->d_name[i] < '0' || entry->d_name[i] > '9') { is_pid = 0; break; }
        }
        if (!is_pid || entry->d_name[0] == '\0') continue;
        snprintf(cmdline_path, sizeof(cmdline_path), "/proc/%s/cmdline", entry->d_name);
        int fd = open(cmdline_path, O_RDONLY);
        if (fd < 0) continue;
        int n = read(fd, cmdline, sizeof(cmdline) - 1);
        close(fd);
        if (n <= 0) continue;
        cmdline[n] = 0;
        if (strcmp(cmdline, "adbd") == 0 ||
            strncmp(cmdline, "/apex/com.android.adbd", 22) == 0 ||
            strncmp(cmdline, "/system/bin/adbd", 16) == 0) {
            closedir(proc);
            char result[64];
            snprintf(result, sizeof(result), "FOUND:pid=%s", entry->d_name);
            return (*env)->NewStringUTF(env, result);
        }
    }
    closedir(proc);
    return (*env)->NewStringUTF(env, "NOT_FOUND");
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_statUsbState(
        JNIEnv *env, jclass clazz) {
    const char *paths[] = {
        "/sys/class/android_usb/android0/state",
        "/sys/class/android_usb/android0/functions",
        "/config/usb_gadget/g1/configs/b.1/strings/0x409/configuration",
        NULL
    };
    for (int i = 0; paths[i]; i++) {
        struct stat st;
        if (stat(paths[i], &st) == 0) {
            int fd = open(paths[i], O_RDONLY);
            if (fd >= 0) {
                char buf[64] = {};
                read(fd, buf, sizeof(buf) - 1);
                close(fd);
                buf[strcspn(buf, "\n")] = 0;
                char result[128];
                snprintf(result, sizeof(result), "FOUND[%s]:%s",
                         strrchr(paths[i], '/') + 1, buf);
                return (*env)->NewStringUTF(env, result);
            }
            return (*env)->NewStringUTF(env, "FOUND:no_read_perm");
        }
    }
    return (*env)->NewStringUTF(env, "NOT_FOUND");
}

JNIEXPORT jstring JNICALL
Java_com_test_adbdetect_DetectorCheck_statAdbKeys(
        JNIEnv *env, jclass clazz) {
    struct stat st;
    if (stat("/data/misc/adb/adb_keys", &st) == 0) {
        char result[64];
        snprintf(result, sizeof(result), "FOUND:size=%ld", (long)st.st_size);
        return (*env)->NewStringUTF(env, result);
    }
    return (*env)->NewStringUTF(env, "NOT_FOUND");
}
