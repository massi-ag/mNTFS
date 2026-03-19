# Test NTFS Disk Images

These images are used by integration tests. They must be created manually
because NTFS formatting requires Windows or ntfs-3g tools.

## Required images for v0.1

| Image | Size | Contents | How to create |
|---|---|---|---|
| `basic.img` | 16MB | hello.txt, Documents/readme.txt, unicode/日本語.txt | Format NTFS on Windows, add files |
| `empty.img` | 8MB | Freshly formatted, no files | Format NTFS on Windows |
| `dirty.img` | 16MB | Same as basic but with dirty journal | Yank VM mid-write |
| `deep-paths.img` | 16MB | 30-level nested directories | Create nested dirs on Windows |

## Creation steps (Windows VM)

1. Open Disk Management
2. Create new VHD (fixed size, specified MB)
3. Initialize as MBR
4. Create simple volume, format NTFS
5. Add required files
6. For dirty image: don't safely eject
7. Copy .vhd, extract raw partition with: `dd if=disk.vhd of=image.img bs=512 skip=<partition_offset>`

## Alternative: mkntfs (if available)

```bash
brew install ntfs-3g-mac  # or ntfs-3g
dd if=/dev/zero of=basic.img bs=1M count=16
mkntfs -F -L "Basic" basic.img
```

Then mount and add files on a Windows VM or use ntfscreate tools.
