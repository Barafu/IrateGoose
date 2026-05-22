# Irate Goose - Virtual Surround Sound

This is quick reminder of application's basics. Full help is available on the homepage above. 

## Quick Start

1. Options tab: Set IR files directory (click Select, then Rescan)
2. Files tab: Select an IR file from the list
3. Click "💾 Create device" button
4. In system sound settings, select "Virtual Surround Sink"

## Download IR files

- Small collection (27Mb) [Direct link](https://d1952d03d5d6-hrir-repository.s3.ru1.storage.beget.cloud/HRIR_collection_small.tar.zstd)
- Full collection (153Mb, includes Small) [Direct link](https://d1952d03d5d6-hrir-repository.s3.ru1.storage.beget.cloud/HRIR_collection_full.tar.zstd)
- Alternative link (mega.nz, if links above don't work) [Link](https://mega.nz/folder/zPx2jAxK#icrUEYHI6St-7m8nUgqcrg)

## Interface overview

Files Tab
---------
- Browse and select IR files for surround sound
- Filter by sample rate: 48000, 44100, 96000, or All
- Search files by name
- View HRTF metadata for selected file

Options Tab
-----------
- Set directory containing WAV files
- Customize virtual device name
- Select output device (Auto or specific audio sink)
- Choose UI theme (light/dark)

Log Tab
-------
- View application events and errors

Managing Devices
----------------
**Create**: Select file, click "💾 Create device"
**Remove**: Click "❌ Remove device"
**Change**: Select new file, click "💾 Update device"
Note: Virtual device works system-wide. Irate Goose doesn't need to run after configuration.

Important Configuration
-----------------------
- Applications must output 7.1 audio (not stereo)
- In games: Use "7.1 Surround" or "Studio Speakers" settings
- Avoid "Headphones" or "Stereo" settings
- Try different IR files to find what works best for you

Troubleshooting
---------------
No virtual device?
- Enable "Show virtual devices" in system sound settings
- Restart audio: systemctl --user restart wireplumber pipewire pipewire-pulse

No sound?
- Confirm virtual device is selected as output
- Check output device selection in Options tab (try "Auto" if specific device isn't working)
- Ensure applications output 7.1 audio
- Try a different IR file

No application icon?
- On Wayland, the app needs a .desktop entry in the start menu to show an icon
- Use your distro's AppImage integration tool

Can't find WAV files?
- Set WAV folder on Options tab


