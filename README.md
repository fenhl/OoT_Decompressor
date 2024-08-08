# OoT_Decoder

---

This is a decoder for Zelda: Ocarina of Time. It will take a compressed ROM of the game, and decompress it into another ROM. This is useful for modifying the game data, as most sources detailing memory offsets are done so for the decompressed version of the game, not the compressed version.

This is essentially the same as another decoder be the name of ndec, but I wanted one of my own, so I made a new one. Mine uses the same algorithm, and some of the same files as ndec.

If you want to play the OoT Randomiser, you'll need a decompressed ROM, so you would want to put a compressed ROM through this program first, then use the randomiser.

Usage: `decompress.exe <INPUT_ROM> [OUTPUT_ROM]`

Or just drag and drop a compressed ROM into decompress.exe
