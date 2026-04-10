'=============================================================================
' QBNex Version Information Module
' Copyright © 2026 thirawat27
' Version: 1.0.0
' Description: Centralized version and build information
'=============================================================================

DIM SHARED VersionMajor AS INTEGER
DIM SHARED VersionMinor AS INTEGER
DIM SHARED VersionPatch AS INTEGER
DIM SHARED VersionString AS STRING
DIM SHARED BuildType AS STRING
DIM SHARED RepositoryURL AS STRING

VersionMajor = 1
VersionMinor = 0
VersionPatch = 0
VersionString$ = "1.0.0"
BuildType$ = "Release"
RepositoryURL$ = "https://github.com/thirawat27/QBNex"

'Project metadata
DIM SHARED ProjectName AS STRING
DIM SHARED ProjectOwner AS STRING
DIM SHARED ProjectYear AS STRING

ProjectName$ = "QBNex"
ProjectOwner$ = "thirawat27"
ProjectYear$ = "2026"

'=============================================================================
' Get full version string
'=============================================================================
FUNCTION get_full_version$ ()
    get_full_version$ = ProjectName$ + " v" + VersionString$ + " (" + BuildType$ + ")"
END FUNCTION

'=============================================================================
' Get copyright notice
'=============================================================================
FUNCTION get_copyright$ ()
    get_copyright$ = "Copyright © " + ProjectYear$ + " " + ProjectOwner$
END FUNCTION

'=============================================================================
' Check if version matches
'=============================================================================
FUNCTION version_matches$ (major AS INTEGER, minor AS INTEGER, patch AS INTEGER)
    IF major = VersionMajor AND minor = VersionMinor AND patch = VersionPatch THEN
        version_matches$ = "-1" 'True
    ELSE
        version_matches$ = "0" 'False
    END IF
END FUNCTION
