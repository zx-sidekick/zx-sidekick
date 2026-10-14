# sdl-notices.awk: finds the notices in SDL3's source files for scripts/sdl-notices.sh, which says
# what is pinned. Prints one record per notice: its number, the file it's pinned from, its
# licence, its first and last lines, separated by \037; writes the notice to <out>/<number>.txt.
function label(t) {
  if (match(t, /SPDX-License-Identifier:[ \t]*[^\n]*/)) {
    l = substr(t, RSTART, RLENGTH); sub(/^SPDX-License-Identifier:[ \t]*/, "", l); sub(/[ \t]+$/, "", l)
    return l
  }
  if (index(t, "BSD 3-Clause")) return "BSD-3-Clause"
  if (index(t, "Sun Microsystems") && index(t, "freely granted")) return "SunPro"
  if (index(t, "At the discretion of the user of this library")) return "BSD-3-Clause"
  if (index(t, "may be used by anyone for any reason")) return "BSD-3-Clause"
  if (index(t, "Permission is granted to anyone to use this software for any purpose")) return "Zlib"
  if (index(t, "Licensed under the MIT license") || index(t, "Permission is hereby granted, free of charge")) return "MIT"
  if (index(t, "Permission to use, copy, modify, distribute, and sell")) return "HPND"
  if (index(t, "under the ZLib license")) return "Zlib"
  return "UNKNOWN"
}
function trimmed(s) { sub(/^[ \t]+/, "", s); return s }
function flush(   i, j, k, text, sub_, s, e, startl, endl, ok, path, nxt) {
  if (nl == 0) return
  text = line[1]; off[1] = 0
  for (i = 2; i <= nl; i++) { off[i] = length(text) + 1; text = text "\n" line[i] }
  done = 0
  for (i = 1; i <= nl; i++) {
    if (i <= done || line[i] !~ /[Cc]opyright[[:space:]]+(\([Cc]\)|©|[0-9])/ || index(line[i], "Sam Lantinga")) continue
    j = i
    if (trimmed(line[i]) ~ /^\/\//) {
      while (j < nl && trimmed(line[j + 1]) ~ /^\/\//) j++
    } else {
      while (j < nl) {
        nxt = line[j + 1]
        if (index(nxt, "*/") || index(nxt, "</copyright>") || nxt ~ /^[ \t]*[*\/#]*[ \t]*[-=*`\/][-=*`\/][-=*`\/]+[ \t]*$/) break
        j++
      }
    }
    while (j > i && line[j] ~ /^[ \t*#\/]*$/) j--
    done = j
    startl = line[i]; endl = line[j]
    sub_ = substr(text, off[i] + 1, off[j] + length(endl) - off[i])
    s = index(text, startl) - 1
    e = (s == off[i]) ? index(substr(text, s + 1), endl) - 1 + s : -1
    ok = (s == off[i] && e == off[j])
    path = substr(file, length(root) + 1)
    if (!(sub_ in seen)) { seen[sub_] = ++count; body[count] = sub_; picked[count] = 0 }
    k = seen[sub_]
    if (ok && !picked[k]) { picked[k] = 1; rep[k] = path; st[k] = startl; en[k] = endl }
  }
  nl = 0
}
FNR == 1 { flush(); file = FILENAME }
{ line[++nl] = $0 }
END {
  flush()
  for (k = 1; k <= count; k++) {
    if (!picked[k]) { print "sdl-notices: a notice can't be pinned:\n" body[k] > "/dev/stderr"; exit 1 }
    printf "%s", body[k] > (out "/" k ".txt"); close(out "/" k ".txt")
    printf "%d\037%s\037%s\037%s\037%s\n", k, rep[k], label(body[k]), st[k], en[k]
  }
}
