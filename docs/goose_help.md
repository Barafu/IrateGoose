IrateGoose - Virtual Surround Sound

Quick Start
-----------
1. Options tab: Set IR files directory (click Select, then Rescan)
2. Files tab: Select an IR file from the list
3. Click "💾 Create device" button
4. In system sound settings, select "Virtual Surround Sink"

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
- Choose UI theme (light/dark)

Log Tab
-------
- View application events and errors

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
- Ensure applications output 7.1 audio
- Try a different IR file

Can't find WAV files?
- Set WAV folder on Options tab
- Or launch with: irate_goose /path/to/wav/files

Managing Devices
----------------
Create: Select file, click "💾 Create device"


Remove: Click "❌ Remove device"


Change: Select new file, click "💾 Create device" again

Command Line
------------
```
irate_goose /path/to/wav/files
irate_goose --install
irate_goose --uninstall
```

Note: Virtual device works system-wide. IrateGoose doesn't need to run after configuration.