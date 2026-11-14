# Floor plans

Each `<name>.toml` here is one drawing. A project shows it with `plan = "<name>"`
in `content/projects.toml`, and `src/floorplan.rs` draws it: walls, doors,
windows, stairs, furniture, room names and areas, dimensions, grid, north arrow
and scale bar, plus a room schedule on the project page.

All measures are **millimetres**, x to the east, **y down** (north is up).
Walls are given by their **centrelines**; room areas are measured from the
rectangles you give them.

## Plan

```toml
wall = 300        # default exterior wall thickness (optional)
partition = 120   # default partition thickness (optional)
column = 400      # column size (optional)

[[level]]         # one or more; levels are drawn side by side
name = "Ground floor"
outline = [[0, 0], [12000, 0], [12000, 9000], [0, 9000]]   # exterior wall, closed
xdims = [0, 3800, 12000]   # extra dimension chain along the top (optional)
ydims = [0, 3000, 9000]    # … and along the left (optional)
grid = { x = [0, 7200], y = [0, 8000] }   # structural grid, bubbles 1 2 3 / A B C (optional)
```

## Items (on a level, or inside a group)

| Key | Each entry |
|---|---|
| `walls` | `[x1, y1, x2, y2]` or `[x1, y1, x2, y2, thickness]`, default `wall` |
| `partitions` | same, default `partition` |
| `columns` | `[x, y]` centre |
| `doors` | `{ at = [x, y], w, run, swing, style }`: `at` is the hinge on the wall centreline, `run` the direction along the wall (`"N" "E" "S" "W"`), `swing` the side it opens to. `style` = `"swing"` (default), `"double"`, `"slide"`, `"opening"` |
| `windows` | `{ at = [x, y], w, run }` |
| `stairs` | `{ rect = [x, y, w, h], dir, steps, kind, label, left }`: `dir` is the way up; `kind = "u"` draws two flights and a half landing (`left = true` puts the first flight on the left) |
| `furniture` | `{ kind, rect = [x, y, w, h], side, seats }`: `side` is the piece's back. Kinds: `bed sofa chair table round desk wc basin bath shower sink hob counter display shelf wardrobe rack lift tree plant curtain car`; anything else is a plain box |
| `rooms` | `{ name, rect or rects, at, floor, style, list, outdoor, kind }`: `floor` = `"tile" "paving" "timber" "hatch"`; `style` = `"room"` (name + area), `"tag"` (name only, accent colour) or `"none"`; `list = false` leaves it out of the schedule; `outdoor = true` lists it without counting it in the floor area; `kind` is the schedule name |
| `labels` | `{ at = [x, y], text }` |
| `entries` | `{ at = [x, y], dir }`: an accent arrow, pointing the way in |

## Groups: repeat and mirror

```toml
[[level.group]]
origins = [[0, 0], [26600, 0]]   # where to place the items (default [[0, 0]])
repeat = [5, 3800, 0]            # count, dx, dy at each origin
mirror_y = 8100                  # also draw everything mirrored about y = 8100
mirror_x = 14000                 # … or about x = 14000
start = 101                      # first number for {n} / {a} in room names
rooms = [{ name = "{n}", kind = "Guest room", rect = [0, 0, 3800, 7200] }]
```

`{n}` in a room name becomes the copy's number, `{a}` its letter (A, B, …).
In the schedule, copies are counted together under `kind`.
