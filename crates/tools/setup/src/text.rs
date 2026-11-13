//! What the setup window says.

macro_rules! texts {
    ($($key:ident: $en:literal;)*) => {
        /// A sentence of the setup window.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key {
            $($key,)*
        }

        impl Key {
            #[cfg(test)]
            const ALL: &'static [Key] = &[$(Key::$key,)*];

            /// The sentence. `{}` stands for a value [`fill`] puts in.
            pub fn text(self) -> &'static str {
                match self {
                    $(Key::$key => $en,)*
                }
            }
        }
    };
}

texts! {
    WindowTitle: "QBNex Setup";
    UninstallTitle: "QBNex Uninstall";
    Back: "Back";
    Next: "Next";
    Install: "Install";
    Remove: "Uninstall";
    Finish: "Finish";
    Cancel: "Cancel";
    Yes: "Yes";
    No: "No";
    Browse: "Browse...";

    WelcomeTitle: "Install QBNex {}";
    WelcomeBody: "A BASIC compiler for QBasic and QB64 programs.\n\nSetup installs QBNex for your user account only. No administrator rights are needed.";
    NoBundle: "This copy of qbnex-setup carries no QBNex files. Download the release installer.";

    LicenseTitle: "License Agreement";
    LicenseSub: "Please read the following before continuing.";
    LicenseNote: "Component licenses are installed in the \"licenses\" folder.";
    Accept: "I accept the agreement";
    Decline: "I do not accept the agreement";

    FolderTitle: "Select Destination Location";
    FolderSub: "Where should QBNex be installed?";
    FolderBody: "Setup will install QBNex into the following folder. To use a different folder, type it or click Browse.";
    SpaceRequired: "Space required: {} MB";
    SpaceFree: "Space available: {} MB";
    Upgrade: "QBNex {} is installed in this folder and will be replaced.";
    UpgradeUnknown: "QBNex is installed in this folder and will be replaced.";
    FolderEmpty: "Type the folder to install into.";
    FolderRelative: "The folder must be a full path, for example {}.";
    FolderIsFile: "A file with this name exists. Choose another folder.";
    FolderNoWrite: "Setup cannot write to this folder: {}";
    FolderNoSpace: "There is not enough free space on this drive.";
    FolderInUse: "The folder already exists and holds other files. Install into it anyway?";

    TasksTitle: "Select Additional Tasks";
    TasksSub: "Which additional tasks should be performed?";
    TaskPath: "Make the qb command available in terminals (PATH)";
    TaskMenu: "Add QBNex to the applications menu";
    TaskDesktop: "Create a desktop shortcut";
    TaskAssociate: "Add \"Compile with QBNex\" to the menu of .bas files";

    ReadyTitle: "Ready to Install";
    ReadySub: "Setup is now ready to install QBNex on your computer.";
    ReadyFolder: "Destination folder:";
    ReadyTasks: "Additional tasks:";
    ReadyNoTasks: "(none)";

    InstallingTitle: "Installing";
    InstallingSub: "Please wait while Setup installs QBNex on your computer.";
    StepPrepare: "Setting the previous version aside...";
    StepExtract: "Extracting files...";
    StepRegister: "Creating shortcuts and settings...";
    StepRemove: "Removing files...";
    Cancelling: "Cancelling and restoring...";
    ConfirmCancel: "Setup is not complete. Exit Setup now?";

    DoneTitle: "Installation complete";
    DoneBody: "QBNex {} has been installed on your computer.";
    DonePath: "Open a new terminal and run: qb --version";
    DoneNoPath: "PATH was not changed. The compiler is: {}";
    OpenReadme: "Open the README";
    OpenFolder: "Open the install folder";
    FailedTitle: "Setup did not finish";
    FailedBody: "QBNex was not installed. Your computer is as it was before Setup started.";
    CancelledBody: "Setup was cancelled. Your computer is as it was before Setup started.";

    RemoveTitle: "Uninstall QBNex";
    RemoveBody: "QBNex will be removed from the following folder. Files you added to it stay.";
    RemovingSub: "Please wait while QBNex is removed from your computer.";
    RemovedTitle: "QBNex was removed";
    RemovedBody: "QBNex was removed from your computer.";
    RemoveFailedTitle: "Uninstall did not finish";
}

/// `text` with its first `{}` replaced by `value`.
pub fn fill(text: &str, value: &str) -> String {
    text.replacen("{}", value, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_is_there_and_placeholders_are_filled() {
        for key in Key::ALL {
            assert!(!key.text().is_empty() && key.text().is_ascii(), "{key:?}");
        }
        assert_eq!(fill("a {} b {}", "x"), "a x b {}");
    }
}