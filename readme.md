# thInk

Basic vector graphics tool in rust.

Uses least squares regressions to convert the user's mouse input to a well-fitted bezier curve.

To-do:
- Refactor code to enable more components to be added
- ~~Add support for splines not just curves~~
- ~~Parallelize least squares or at the very least run it async~~ (finished! splines are generated in parallel!)
- Add parley for text boxes
