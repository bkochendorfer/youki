use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Clap;

use crate::cgroups;
use crate::container::Container;
use crate::utils;

#[derive(Clap, Debug)]
pub struct Delete {
    container_id: String,
    // forces deletion of the container.
    #[clap(short, long)]
    force: bool,
}

impl Delete {
    pub fn exec(&self, root_path: PathBuf, systemd_cgroup: bool) -> Result<()> {
        log::debug!("start deleting {}", self.container_id);
        // state of container is stored in a directory named as container id inside
        // root directory given in commandline options
        let container_root = root_path.join(&self.container_id);
        if !container_root.exists() {
            bail!("{} doesn't exist.", self.container_id)
        }
        // load container state from json file, and check status of the container
        // it might be possible that delete is invoked on a running container.
        log::debug!("load the container from {:?}", container_root);
        let container = Container::load(container_root)?.refresh_status()?;
        if container.can_delete() {
            if container.root.exists() {
                nix::unistd::chdir(&PathBuf::from(&container.state.bundle))?;
                let config_absolute_path = &PathBuf::from(&container.state.bundle)
                    .join(Path::new("config.json"))
                    .to_string_lossy()
                    .to_string();
                log::debug!("load spec from {:?}", config_absolute_path);
                let spec = oci_spec::Spec::load(config_absolute_path)?;
                log::debug!("spec: {:?}", spec);

                // remove the directory storing container state
                log::debug!("remove dir {:?}", container.root);
                fs::remove_dir_all(&container.root)?;

                let cgroups_path =
                    utils::get_cgroup_path(&spec.linux.unwrap().cgroups_path, container.id());

                // remove the cgroup created for the container
                // check https://man7.org/linux/man-pages/man7/cgroups.7.html
                // creating and removing cgroups section for more information on cgroups
                let cmanager =
                    cgroups::common::create_cgroup_manager(cgroups_path, systemd_cgroup)?;
                cmanager.remove()?;
            }
            std::process::exit(0)
        } else {
            bail!(
                "{} could not be deleted because it was {:?}",
                container.id(),
                container.status()
            )
        }
    }
}
