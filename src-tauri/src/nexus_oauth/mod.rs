// Nexus OAuth module — barrel re-exports all sub-modules
#![allow(unused_imports)]

pub mod types;
pub mod pkce;
pub mod flow;
pub mod profile;
pub mod nxm;

// Re-export types
pub use types::{
    OAuthTokenResponse, NexusUserInfoResponse, NexusUserNested, FetchedProfile,
    ModBasicInfo, NxmLinkInfo, NxmModMetadata, NxmDownloadProgressEvent,
};

// Re-export PKCE helpers
pub use pkce::{get_client_id, get_client_secret, get_redirect_uri, generate_pkce};

// Re-export OAuth flow
pub use flow::{
    start_oauth_flow, handle_oauth_callback, refresh_access_token,
    ensure_valid_nexus_token, decode_jwt_payload,
};

// Re-export profile & GraphQL helpers
pub use profile::{
    fetch_user_profile, fetch_user_endorsements, fetch_user_tracked_mods,
    fetch_user_authored_mods, batch_fetch_legacy_mods_info,
};

// Re-export NXM parser and downloaders
pub use nxm::{
    parse_nxm_url, fetch_nxm_direct_download_url, fetch_nxm_mod_metadata,
    fetch_nxm_file_details, download_file_to_temp_with_progress, download_file_to_temp,
};
