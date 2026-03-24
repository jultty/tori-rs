use crate::log;

pub fn load() -> Configuration {
    log::elog("Loading configuration");

    // TODO A3.1. Before parsing the user arguments, a configuration file at
    //       $XDG_CONFIG_DIR/tori/tori.conf MUST be read for a line such as:
    //       'su_command = doas'.
    // TODO A3.2. If this line is not found, the su_command MUST default to 'su -c'.
    // TODO A3.3. If it is found, the su_command used MUST be whatever was specified.
    // TODO A3.4. Whatever su_command MUST be validated once for presence at the path
    //       provided or obtained from $PATH and filesystem permission to execute

    Configuration {
        su_command: String::default(),
    }
}

#[derive(Debug)]
pub struct Configuration {
    su_command: String,
}
