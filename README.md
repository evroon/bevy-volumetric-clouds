<h1 align="center">
    Bevy Volumetric Clouds
</h1>

<p align="center">
  <a href="https://crates.io/crates/bevy-volumetric-clouds"
    ><img
      src="https://img.shields.io/crates/v/bevy-volumetric-clouds"
      alt="crate on crates.io"
  /></a>
  <a href="https://docs.rs/bevy-volumetric-clouds"
    ><img
      src="https://img.shields.io/docsrs/bevy-volumetric-clouds"
      alt="docs on docs.rs"
  /></a>
  <a href="https://github.com/evroon/bevy-volumetric-clouds/actions"
    ><img
      src="https://img.shields.io/github/actions/workflow/status/evroon/bevy-volumetric-clouds/ci.yml"
      alt="build status"
  /></a>
</p>

![clouds](https://github.com/evroon/bevy-volumetric-clouds/raw/master/.github/screenshots/clouds.jpg)

This is a plugin for [Bevy](https://github.com/bevyengine/bevy) that renders volumetric clouds
using the method of Horizon Zero Dawn by Guerilla Games (see [credits](#credits)).

## Usage

Run `cargo add bevy-volumetric-clouds` and simply add `CloudsPlugin` to your Bevy App like this:

```rust ignore
use bevy_volumetric_clouds::CloudsPlugin;

app.add_plugins(CloudsPlugin);
```

Look at [the minimal example](examples/minimal.rs) for a working example.

The [the demo example](examples/demo.rs) features a usable demo where you can move the camera around
and use a UI to change the configuration of the cloud rendering
(if you run it with the `fly_camera` and `debug` features):

```sh
cargo run --example demo --features fly_camera,debug
```

## Limitations

A few limitations apply for now and hopefully get fixed in the future:

- There is no integration with Bevy's internal atmosphere rendering yet, this plugin uses a simple
  sky rendering function.
- The clouds are drawn on a skybox that does not take the depth buffer into account yet. Therefore,
  it's not yet possible to "fly" into the clouds, the clouds are only visible from ground-level.
- For now the clouds render resolution is set to 1920x1080 and can't be changed.
  Usually clouds are rendered at a lower resolution than screen resolution so you likely won't need
  a higher resolution anyway but changing the resolution should be possible in the future.

## Crate features

There are a few features:

- `debug`: enables an `egui` UI that allows you to tweak shader uniforms (parameters) in-game.
- `fly_camera`: adds a `fly_camera` module that controls the camera using keyboard and mouse.

## Bevy version compatibility

| bevy | bevy-volumetric-clouds |
|------|------------------------|
| 0.17 | 0.1.*                  |

## Credits

1. "The real-time volumetric cloudscapes of Horizon Zero Dawn" by Andrew Schneider and Nathan Vos ([article](https://www.guerrilla-games.com/read/the-real-time-volumetric-cloudscapes-of-horizon-zero-dawn))
2. "Physically Based Sky, Atmosphere and Cloud Rendering in Frostbite" by Sébastien Hillaire ([pdf](https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/s2016-pbs-frostbite-sky-clouds-new.pdf))

## License

Licensed under [MIT](https://choosealicense.com/licenses/mit/), see [LICENSE](./LICENSE).
