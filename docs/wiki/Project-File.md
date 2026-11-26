🌐 **English** · [Polski](Plik-projektu) · [简体中文](Project-File-ZH) · [한국어](Project-File-KO)

# Project File

BaseCreator saves **`.cre`**, the project file that LightCreator, MillCreator and PenCreator write too: one JSON file with the drawing, the description and **one section per program** (`draw`, `laser`, `mill`, `pen`). Each program writes its own section and keeps the others untouched, so a project passes between programs without losing anything.

The `draw` section holds the page and the 64 layers (name, colour, shown or not).

* A project made here opens in the other programs with its drawing and description. They keep `draw`.
* A project made there opens here with its drawing. It gets the default layers and the page of the last new drawing, made bigger when the drawing does not fit. Its machine settings stay in the file after saving here.
* A file with a newer format of `draw` is refused, with a request to update the program.

## Creator and contributors

The file also records who made the project and who worked on it, in the block `authorship`:

* **Creator**: the person who saved the project first.
* **Contributors**: everyone else who saved it, each added once. The creator is kept with the name, the e-mail and the other field; a contributor (and an edit) with the name and the e-mail only, and with the name only when the e-mail is the creator's.
* **Edits**: the saves by hand (the last 200), at most one a minute for the same person and program. Automatic saves add no edit.

Each entry has the name, the e-mail and a free text (a company, a website…), the time (UTC) and the program with its version. They are set in **Settings → Program options**, in the **Author** part: **Your name**, **Email** and **Other**. They are kept in one file of the shared folder (`author.json`), so they are the same in BaseCreator, LightCreator, MillCreator and PenCreator. Without a name or an e-mail, a save records the time and the program but no person.

**File → Project information…** shows the file (name, folder, size, when it was created and changed), the authors (when the project was created, its creator with the e-mail and the other field in rows of their own, the contributors with their e-mail) and a table of the edits: time, person, program.

A project that has a file is saved to it by itself about 20 seconds after a change. See [Settings and Files](Settings-and-Files).

## Password protection

A project can be protected with passwords: in **Project setup…** of LightCreator, MillCreator and PenCreator, and in **File → Page setup…** of BaseCreator, part **Protection**.

* **Owner password**: everything (editing, copying, exporting, changing or removing the protection).
* **Open password** (optional): the project can be opened, looked at, turned into G-code and run on the machine, and the material and the layer settings (speed, power, passes, depth, bit, pen) can be changed, but the drawing cannot be edited, copied, exported as a drawing (SVG, DXF, Illustrator, LightBurn …) or printed. The built-in web page shows no drawing then.
* With only an owner password, nobody else can open the file. Without a password the file cannot be read at all, and a **forgotten password cannot be recovered**.
* A project opened with the open password can be unlocked with the owner password, without opening it again: in the **Protection** part of the setup (**Owner password**, **Unlock**) or with **Unlock with the owner password…** in the menu of its tab (right click). That menu also has **Project information…** and the setup of the project.

The file stays a JSON text with the key `protected_cre`; the project in it is encrypted with **XChaCha20-Poly1305** under a random 256-bit key, which is stored once for every password, wrapped with a key made from the password by **Argon2id** (64 MiB, 3 passes, a salt of its own). Every save uses a new nonce; a changed file does not open. Passwords need at least 8 characters, and the open password must differ from the owner's.

What the open password forbids is enforced by the programs, as with the owner password of a PDF: whoever has it can, in principle, decrypt the project with other tools. Keep the owner password for yourself.
