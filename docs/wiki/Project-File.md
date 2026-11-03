🌐 **English** · [Polski](Plik-projektu)

# Project File

BaseCreator saves **`.cre`**, the project file that LightCreator, MillCreator and PenCreator write too: one JSON file with the drawing, the description and **one section per program** (`draw`, `laser`, `mill`, `pen`). Each program writes its own section and keeps the others untouched, so a project passes between programs without losing anything.

The `draw` section holds the page and the 64 layers (name, colour, shown or not).

* A project made here opens in the other programs with its drawing and description. They keep `draw`.
* A project made there opens here with its drawing. It gets the default layers and the page of the last new drawing, made bigger when the drawing does not fit. Its machine settings stay in the file after saving here.
* A file with a newer format of `draw` is refused, with a request to update the program.

A project that has a file is saved to it by itself about 20 seconds after a change. See [Settings and Files](Settings-and-Files).
