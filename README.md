# Timewarrior Timesheet Report

This script is intended to be run as a [timewarrior](https://timewarrior.net)
extension report.

## Install

Copy the timesheet.py script to the $TIMEWARRIORDB/extension/ folder, then run
it with `timew report timesheet <range>` to see a timesheet for the
specified interval.

Since this report is for entering weekly timesheets, the report range should
be a Mon-Sun week interval (:week, :lastweek, etc..).
