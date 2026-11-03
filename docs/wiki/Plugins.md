🌐 **English** · [Polski](Wtyczki)

# Plugins

Plugins add things without changing the program. They are shared by BaseCreator, LightCreator, MillCreator and PenCreator: a plugin lives in the folder `plugins` inside the shared data folder (`baseCreator`), and each program uses the parts that fit it.

**Settings → Plugins…** opens the window: install a plugin from a `.zip`, switch plugins on and off, remove them, open the folder.

What works here:

* **Importers:** **File → Import → Import with a plugin…**, drag and drop, or **File → Open…** on a file whose extension a plugin reads. The shapes are added to the drawing.
* **Generators:** **File → Import → Generate shape…** (also **Insert → Generate shape…**) shows the form of the generator and adds the shapes it makes.
* **Clipart and translations.**

A plugin with **G-code postprocessors** (`gcode-tools`, `gcode-dialects`) can be installed, but does nothing here, as there is no G-code.

**Safety.** Plugin scripts (Rhai) run in a sandbox without access to files, the network or the system, with a time and memory limit. Install only plugins you trust.

## Official plugins

**Settings → Plugins… → Official plugins** shows the list kept in [baseCreator-plugins](https://github.com/michalsarna/baseCreator-plugins). **Install** downloads a plugin, with its checksum checked, into the shared plugins folder.

Writing your own: the [guide for authors](https://github.com/michalsarna/baseCreator-plugins/blob/main/docs/GUIDE.md) and [PLUGINS.md](https://github.com/michalsarna/baseCreator/blob/main/docs/PLUGINS.md).
