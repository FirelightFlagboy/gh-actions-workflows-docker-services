/^.+$/ {
  if (unreleased_header)
    start_printing=1
}
/## Unreleased/ { unreleased_header=1 }
/marker-end-of-unreleased-change/ { exit }

{
  if (start_printing)
    print
}
