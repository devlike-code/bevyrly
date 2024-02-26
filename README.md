# Bevyrly 

![bevyrly3](https://github.com/devlike-code/bevyrly/assets/2606844/ad44396b-ebb0-4a0c-bd74-7e72adc0cbf7)

**Bevy** is **rly** useful, but requires some hygiene! Pronounced as /ˈbɛvə(ɹ)li/, derives from Old English, combining *befer* ("beaver") and *leah* (\"clearing\").

This tool allows you to open one or more notebooks in Visual Studio and go hog wild exploring your systems. Instead of tracing them through files, find them via their arguments and then save that as a compilable notebook that will auto-update.

## Features

To start using Bevyrly (just like Beverly), install this extension, and then open the palette (Ctrl+Shift+P) and type `Bevyrly: New Notebook`. Once inside, write `?` and run it to get full docs, just like what you have below.

### Search control
- `&Transform`: find all systems that include `Query<&Transform>` within it
- `*Transform`: find all systems that include `Query<&mut Transform>` within it
- `#Config`: find all systems that include `Res<Config>` within it
- `$Config`: find all systems that include `ResMut<Config>` or `NonSendMut<Config>` within it
- `<ShipFireEvent`: find all systems that include `EventReader<ShipFireEvent>` within it
- `>ShipFireEvent`: find all systems that include `EventWriter<ShipFireEvent>` within it
- `+Tag`: find all systems that include `With<Tag>` within it
- `-Tag`: find all systems that include `Without<Tag>` within it
- `JustText`: will match any of the above (might yield a *lot* of content)

### Output control
- `?`: prints this documentation
- `my prompt goes here`: find and print locations of all systems that mention 'my', 'prompt', 'goes', and 'here'
- `:my prompt goes here`: find and print declaration for all systems that mention 'my', 'prompt', 'goes', and 'here'

### Examples
- `:&Transform >ShipFireEvent +Player`: prints full function declarations for any system that queries the `Transform` component immutably, accesses `EventWriter<ShipFireEvent>`, and has a `With<Player>`.
- `+Player -Player`: prints linkable locations to all the systems that require `With<Player>` and `Without<Player>` (possibly in different arguments)
- `Foo Bar`: prints locations of all the systems that have the strings `Foo` and `Bar` <i>anywhere</i> in their arguments (including resources, components, etc.)

## How Does It Work

Bevyrly analyzes your code whenever you open a new notebook. It takes the arguments of the systems you use and makes a catalog of the different kinds of resources, components, etc. mapped onto the systems they are used in. When you query Bevyrly, it parses your prompt and intersects the different mappings to get you exactly what you want. _There is no AI used in Bevyrly, and never will be._

## Known Issues

- We don't distinguish things within the same _argument_ of a system, so for example, you can't say `A, B` to mean "give me a system that has one argument with both `A` and `B`.
- Sometimes, the highlighting fails without obvious reason.

## Release Notes

### 0.0.2

Initial release of Bevyrly, a weekend project that I'll need going into a large project

**Enjoy!**
