## packwiz update
Update an external file (or all external files) in the modpack

packwiz update [name] [flags]
Options
  -a, --all    Update all external files
  -h, --help   help for update
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results

## packwiz unpin
Unpin a file so it receives updates

packwiz unpin [flags]
Options
  -h, --help   help for unpin
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results


## packwiz pin
Pin a file so it does not get updated automatically

packwiz pin [flags]
Options
  -h, --help   help for pin
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results