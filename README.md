# recipe-scale

A recipe is written for a fixed number of servings. You want a different
number. Every quantity in the ingredient list has to change, and doing that
by hand means re-deriving fractions like `1 1/2 cups -> 2 1/4 cups` for every
single line without making an arithmetic mistake somewhere along the way.

recipe-scale does that one job: read a recipe, scale every quantity by
`to / from`, print the result. Everything that isn't a quantity - section
headers, oven temperatures, instructions - passes through untouched.

## Usage

```
$ cat lasagna.txt
Serves 4
1 1/2 cups ricotta
2 cups shredded mozzarella
1/2 tsp salt
3 eggs
Bake at 375F for 45 minutes.

$ recipe-scale --from 4 --to 6 lasagna.txt
Serves 4
2 1/4 cups ricotta
3 cups shredded mozzarella
3/4 tsp salt
4 1/2 eggs
Bake at 375F for 45 minutes.
```

If no file is given, it reads from stdin, so it works in a pipeline:

```
$ cat lasagna.txt | recipe-scale --from 4 --to 12 > party-size.txt
```

Quantities can be whole numbers, decimals, simple fractions (`1/2`), or
mixed numbers (`1 1/2`). A line that doesn't start with a number - like the
`Serves 4` header above - is left as-is; only ingredient amounts are scaled.

If the recipe's first line is a `Serves`/`Yield` header, `--from` can be
left out and the serving count is read from it instead:

```
$ recipe-scale --to 6 lasagna.txt
Serves 4
2 1/4 cups ricotta
...
```

If `--from` is omitted and the first line isn't a recognizable header,
recipe-scale exits with an error rather than guessing.

## Why streaming matters here

The input is read and processed one line at a time (`BufRead::lines`), and
each output line is written before the next one is read. A recipe file is
normally tiny, but the same code path handles a multi-thousand-line file (a
scraped cookbook, a shared meal-prep spreadsheet exported as text) without
ever holding the whole thing in memory - only the current line.

## Build

```
cargo build --release
```

No third-party dependencies - standard library only.

## License

MIT, see LICENSE.
