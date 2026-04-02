# ark-story-plaintext

Compiles Arknights story scripts into readable plaintext.

## Usage

1. Find stories at [ArknightsGameData](https://github.com/Kengxxiao/ArknightsGameData/tree/master/zh_CN/gamedata/story/activities)
2. Place `.txt` files into `inputs/`, prefixed with `[index].` to set reading order (e.g. `01. level_xxx.txt`, `02. level_xxx.txt`). Files with `_st\d\d.txt` suffixes are interludes — place them where they belong in the sequence.
3. Run:

```bash
cargo run
```

Output is written to `outputs/story.txt`.
