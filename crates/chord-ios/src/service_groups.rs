//! The background services a slim simulator does not start, in the groups Settings lists: a Rust port of the service list of simslim 0.13.0 (https://github.com/mobai-app/simslim). Core services and the ones that wedge a simulator when off (`assistantd`, `gamed`, `sharingd` and the rest simslim never disables) are not listed.
//!
//! simslim is MIT licensed:
//!
//! Copyright (c) 2026 Interlap
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use serde::Serialize;
use specta::Type;

/// The launchd services that start or stay off together.
pub struct Subgroup {
    /// Its group's id when the group has only this one.
    pub id: &'static str,
    pub name: &'static str,
    /// What stops working while it is off.
    pub effect: &'static str,
    /// An estimate: the group's measured idle memory on an iOS 26.5 clean boot, split across its subgroups.
    pub memory_mb: u32,
    pub(crate) labels: &'static [&'static str],
}

/// A group of subgroups the user opens in Settings, with one switch for them all.
pub struct ServiceGroup {
    pub id: &'static str,
    pub name: &'static str,
    /// What stops working while the whole group is off.
    pub effect: &'static str,
    /// Whether its subgroups start on: App Store, Push & Media and Safari Sync & Web Services (universal links).
    pub kept_by_default: bool,
    pub(crate) subgroups: &'static [Subgroup],
}

/// One subgroup of Settings' Simulators section with the state of its switch.
#[derive(Serialize, Type, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubgroupRow {
    pub id: &'static str,
    pub name: &'static str,
    pub effect: &'static str,
    pub memory_mb: u32,
    /// The user's switch for it, else the switch of its group, else the group's default.
    pub kept: bool,
}

/// One row of Settings' Simulators section: a group with the state of its switch.
#[derive(Serialize, Type, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRow {
    pub id: &'static str,
    pub name: &'static str,
    pub effect: &'static str,
    /// The sum of its subgroups'.
    pub memory_mb: u32,
    /// On when every subgroup is kept, off when none is, `None` when they differ.
    pub kept: Option<bool>,
    /// Empty for a group with one coherent function, which has no rows to open.
    pub subgroups: Vec<SubgroupRow>,
}

pub static GROUPS: &[ServiceGroup] = &[
    ServiceGroup {
        id: "widgets",
        name: "Widgets & Wallpaper",
        effect: "Home and Lock Screen widgets, wallpaper posters, and Live Activities stop updating.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "widgets.wallpaper",
                name: "Wallpaper & Posters",
                effect: "Lock Screen wallpaper posters stop rendering.",
                memory_mb: 325,
                labels: &["com.apple.PosterBoard"],
            },
            Subgroup {
                id: "widgets.widgets",
                name: "Widgets & Live Activities",
                effect: "Home and Lock Screen widgets and Live Activities stop updating.",
                memory_mb: 350,
                labels: &[
                    "com.apple.chronod",
                    "com.apple.liveactivitiesd",
                    "com.apple.navd",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "siri",
        name: "Siri & Intelligence",
        effect: "Siri, speech features, and Apple Intelligence services are unavailable.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "siri.siri",
                name: "Siri & Speech",
                effect: "Siri, dictation, and speech features are unavailable.",
                memory_mb: 120,
                labels: &[
                    "com.apple.assistant_cdmd",
                    "com.apple.assistant_service",
                    "com.apple.siriactionsd",
                    "com.apple.siriinferenced",
                    "com.apple.siriknowledged",
                    "com.apple.sirittsd",
                    "com.apple.siri.context.service",
                    "com.apple.siri.acousticsignature",
                    "com.apple.corespeechd",
                    "com.apple.voiced",
                    "com.apple.voicebankingd",
                    "com.apple.speechmodeltrainingd",
                ],
            },
            Subgroup {
                id: "siri.intelligence",
                name: "Apple Intelligence",
                effect: "On-device models and Apple Intelligence features are unavailable.",
                memory_mb: 100,
                labels: &[
                    "com.apple.intelligenceplatformd",
                    "com.apple.intelligencecontextd",
                    "com.apple.intelligenceflowd",
                    "com.apple.intelligencetasksd",
                    "com.apple.generativeexperiencesd",
                    "com.apple.knowledgeconstructiond",
                    "com.apple.modelcatalogd",
                    "com.apple.modelmanagerd",
                    "com.apple.mlhostd",
                    "com.apple.mlruntimed",
                ],
            },
            Subgroup {
                id: "siri.suggestions",
                name: "Suggestions & Language",
                effect: "Siri Suggestions and natural-language analysis stop working.",
                memory_mb: 45,
                labels: &[
                    "com.apple.naturallanguaged",
                    "com.apple.textunderstandingd",
                    "com.apple.suggestd",
                    "com.apple.parsecd",
                    "com.apple.parsec-fbf",
                    "com.apple.proactiveeventtrackerd",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "search",
        name: "Spotlight & Search",
        effect: "Spotlight and Settings search return no results, and Recents in Files and the document picker fails to load.",
        kept_by_default: false,
        subgroups: &[Subgroup {
            id: "search",
            name: "Spotlight & Search",
            effect: "Spotlight and Settings search return no results, and Recents in Files and the document picker fails to load.",
            memory_mb: 50,
            labels: &[
                "com.apple.searchd",
                "com.apple.searchtoold",
                "com.apple.spotlightknowledged",
                "com.apple.spotlightknowledged.updater",
                "com.apple.corespotlightservice",
            ],
        }],
    },
    ServiceGroup {
        id: "icloud",
        name: "iCloud & Apple Account",
        effect: "iCloud sync, Apple Account, Keychain, and backup workflows will not work.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "icloud.sync",
                name: "iCloud Sync",
                effect: "CloudKit, iCloud Drive, and iCloud Photos sync will not work.",
                memory_mb: 45,
                labels: &[
                    "com.apple.cloudd",
                    "com.apple.cloudphotod",
                    "com.apple.ckdiscretionaryd",
                    "com.apple.cloudsettingssyncagent",
                    "com.apple.bird",
                    "com.apple.syncdefaultsd",
                    "com.apple.protectedcloudstorage.protectedcloudkeysyncing",
                    "com.apple.icloudmailagent",
                    "com.apple.icloudsubscriptionoptimizerd",
                ],
            },
            Subgroup {
                id: "icloud.account",
                name: "Apple Account",
                effect: "Sign in with an Apple Account and account-based services will not work.",
                memory_mb: 30,
                labels: &[
                    "com.apple.appleaccountd",
                    "com.apple.appleaccounttransparencyd",
                    "com.apple.appleidsetupd",
                    "com.apple.akd",
                    "com.apple.amsaccountsd",
                    "com.apple.amsengagementd",
                    "com.apple.amsondevicestoraged",
                    "com.apple.communicationtrustd",
                ],
            },
            Subgroup {
                id: "icloud.keychain",
                name: "Keychain & Backup",
                effect: "iCloud Keychain and backup workflows will not work.",
                memory_mb: 25,
                labels: &[
                    "com.apple.cdpd",
                    "com.apple.sosd",
                    "com.apple.SecureBackupDaemon",
                    "com.apple.TrustedPeersHelper",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "store",
        name: "App Store, Push & Media",
        effect: "Remote push notifications and StoreKit or App Store testing will not work.",
        kept_by_default: true,
        subgroups: &[
            Subgroup {
                id: "store.storekit",
                name: "App Store & StoreKit",
                effect: "StoreKit purchases and App Store testing will not work.",
                memory_mb: 40,
                labels: &[
                    "com.apple.appstored",
                    "com.apple.appstorecomponentsd",
                    "com.apple.itunesstored",
                    "com.apple.storekitd",
                    "com.apple.amsaccountsd",
                    "com.apple.amsengagementd",
                    "com.apple.amsondevicestoraged",
                    "com.apple.passd",
                    "com.apple.financed",
                ],
            },
            Subgroup {
                id: "store.push",
                name: "Push Notifications",
                effect: "Remote push notifications will not arrive.",
                memory_mb: 15,
                labels: &["com.apple.apsd"],
            },
            Subgroup {
                id: "store.media",
                name: "Media & Subscriptions",
                effect: "Music, video subscriptions, and iTunes cloud media will not work.",
                memory_mb: 25,
                labels: &[
                    "com.apple.itunescloudd",
                    "com.apple.videosubscriptionsd",
                    "com.apple.assetsubscriptiond",
                    "com.apple.musicd",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "pim",
        name: "Mail, Calendar & Contacts",
        effect: "Contacts, Calendar, Reminders, and Mail-backed pickers or sync may fail.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "pim.mail",
                name: "Mail & Account Sync",
                effect: "Mail and Exchange or account data sync may fail.",
                memory_mb: 35,
                labels: &[
                    "com.apple.email.maild",
                    "com.apple.exchangesyncd",
                    "com.apple.dataaccess.dataaccessd",
                ],
            },
            Subgroup {
                id: "pim.calendar",
                name: "Calendar & Reminders",
                effect: "Calendar and Reminders access may fail.",
                memory_mb: 20,
                labels: &["com.apple.calaccessd", "com.apple.remindd"],
            },
            Subgroup {
                id: "pim.contacts",
                name: "Contacts",
                effect: "Contacts access and the contact picker may fail.",
                memory_mb: 25,
                labels: &[
                    "com.apple.contactsd",
                    "com.apple.contacts.postersyncd",
                    "com.apple.peopled",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "web",
        name: "Safari Sync & Web Services",
        effect: "Universal links and Safari sync or background web services will not work.",
        kept_by_default: true,
        subgroups: &[
            Subgroup {
                id: "web.links",
                name: "Universal Links",
                effect: "Universal links will not open the app.",
                memory_mb: 10,
                labels: &["com.apple.swcd"],
            },
            Subgroup {
                id: "web.safari",
                name: "Safari Sync & Safety",
                effect: "Safari bookmarks, history, and safe browsing checks will not sync or update.",
                memory_mb: 25,
                labels: &[
                    "com.apple.SafariBookmarksSyncAgent",
                    "com.apple.Safari.History",
                    "com.apple.Safari.passwordbreachd",
                    "com.apple.Safari.SafeBrowsing.Service",
                    "com.apple.safarifetcherd",
                    "com.apple.WebBookmarks.webbookmarksd",
                ],
            },
            Subgroup {
                id: "web.push",
                name: "Web Push & Privacy",
                effect: "Web push and WebKit ad attribution or privacy services will not work.",
                memory_mb: 15,
                labels: &[
                    "com.apple.webkit.adattributiond",
                    "com.apple.webkit.webpushd",
                    "com.apple.webprivacyd",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "family",
        name: "Family & Screen Time",
        effect: "Family Sharing, Screen Time, and usage tracking stop working.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "family.sharing",
                name: "Family Sharing",
                effect: "Family Sharing and Ask to Buy stop working.",
                memory_mb: 25,
                labels: &[
                    "com.apple.familycircled",
                    "com.apple.familynotification",
                    "com.apple.askpermissiond",
                    "com.apple.asktod",
                ],
            },
            Subgroup {
                id: "family.screentime",
                name: "Screen Time",
                effect: "Screen Time, Family Controls, and usage tracking stop working.",
                memory_mb: 40,
                labels: &[
                    "com.apple.FamilyControlsAgent",
                    "com.apple.ScreenTimeAgent",
                    "com.apple.ScreenTimeSettingsAgent",
                    "com.apple.UsageTrackingAgent",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "health",
        name: "Health, Home & Fitness",
        effect: "HealthKit, HomeKit, and Fitness integrations will not work.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "health.health",
                name: "Health",
                effect: "HealthKit data and Health records will not work.",
                memory_mb: 60,
                labels: &[
                    "com.apple.healthd",
                    "com.apple.healthappd",
                    "com.apple.healthcontentd",
                    "com.apple.healtheventsd",
                    "com.apple.healthrecordsd",
                    "com.apple.finhealthd",
                ],
            },
            Subgroup {
                id: "health.home",
                name: "Home",
                effect: "HomeKit accessories and automations will not work.",
                memory_mb: 40,
                labels: &["com.apple.homed", "com.apple.homeeventsd"],
            },
            Subgroup {
                id: "health.fitness",
                name: "Fitness",
                effect: "Fitness workouts, coaching, and activity sharing will not work.",
                memory_mb: 35,
                labels: &[
                    "com.apple.fitcore",
                    "com.apple.fitcore.session",
                    "com.apple.fitnesscoachingd",
                    "com.apple.fitnessintelligenced",
                    "com.apple.activityawardsd",
                    "com.apple.activitysharingd",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "photos",
        name: "Photos & Media Analysis",
        effect: "Photo picker, Photos-library workflows, and media analysis may fail.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "photos.analysis",
                name: "Media Analysis",
                effect: "Photo and media analysis, such as faces and scenes, stops.",
                memory_mb: 35,
                labels: &[
                    "com.apple.photoanalysisd",
                    "com.apple.photosface",
                    "com.apple.mediaanalysisd",
                    "com.apple.mediaanalysisd.service",
                ],
            },
            Subgroup {
                id: "photos.library",
                name: "Photos Library",
                effect: "The photo picker and Photos-library workflows may fail.",
                memory_mb: 25,
                labels: &[
                    "com.apple.mediastream.mstreamd",
                    "com.apple.medialibraryd",
                    "com.apple.assetsd",
                    "com.apple.assetsd.nebulad",
                ],
            },
        ],
    },
    ServiceGroup {
        id: "apps",
        name: "News, Weather, Maps & Games",
        effect: "News, Weather, Maps background data, and game-controller services are unavailable.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "apps.news",
                name: "News",
                effect: "News background data and assets are unavailable.",
                memory_mb: 25,
                labels: &["com.apple.newsd", "com.apple.jetpackassetd"],
            },
            Subgroup {
                id: "apps.weather",
                name: "Weather",
                effect: "Weather background data is unavailable.",
                memory_mb: 15,
                labels: &["com.apple.weatherd"],
            },
            Subgroup {
                id: "apps.maps",
                name: "Maps",
                effect: "Maps sync, push, geo corrections, and map snapshots are unavailable.",
                memory_mb: 25,
                labels: &[
                    "com.apple.Maps.mapssyncd",
                    "com.apple.Maps.mapspushd",
                    "com.apple.Maps.geocorrectiond",
                    "com.apple.maps.destinationd",
                    "com.apple.MapKit.SnapshotService",
                ],
            },
            Subgroup {
                id: "apps.games",
                name: "Games",
                effect: "Game Center saves and game-controller services are unavailable.",
                memory_mb: 20,
                labels: &[
                    "com.apple.gamesaved",
                    "com.apple.GameController.gamecontrollerd",
                ],
            },
            Subgroup {
                id: "apps.tips",
                name: "Tips",
                effect: "Tips content is unavailable.",
                memory_mb: 5,
                labels: &["com.apple.tipsd"],
            },
        ],
    },
    ServiceGroup {
        id: "messaging",
        name: "Messaging & FaceTime",
        effect: "iMessage, FaceTime, and related identity services will not work.",
        kept_by_default: false,
        subgroups: &[Subgroup {
            id: "messaging",
            name: "Messaging & FaceTime",
            effect: "iMessage, FaceTime, and related identity services will not work.",
            memory_mb: 60,
            labels: &[
                "com.apple.identityservicesd",
                "com.apple.ids_simd",
                "com.apple.imautomatichistorydeletionagent",
                "com.apple.imcore.imtransferagent",
                "com.apple.imdpersistence.IMDPersistenceAgent",
                "com.apple.facetimemessagestored",
                "com.apple.telephonyutilities.callservicesd",
            ],
        }],
    },
    ServiceGroup {
        id: "connectivity",
        name: "Sharing & Device Connectivity",
        effect: "AirDrop, Continuity, CarPlay, Watch, and Find My connectivity will not work.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "connectivity.watch",
                name: "Watch & Companion",
                effect: "Pairing and syncing with an Apple Watch will not work.",
                memory_mb: 15,
                labels: &["com.apple.companiond", "com.apple.wcd"],
            },
            Subgroup {
                id: "connectivity.airdrop",
                name: "AirDrop & Continuity",
                effect: "AirDrop, Handoff, and Continuity will not work.",
                memory_mb: 15,
                labels: &["com.apple.rapportd", "com.apple.announced"],
            },
            Subgroup {
                id: "connectivity.carplay",
                name: "CarPlay",
                effect: "CarPlay will not connect.",
                memory_mb: 8,
                labels: &["com.apple.carkitd"],
            },
            Subgroup {
                id: "connectivity.findmy",
                name: "Find My",
                effect: "Find My location sharing will not work.",
                memory_mb: 10,
                labels: &["com.apple.findmy.findmylocated"],
            },
            Subgroup {
                id: "connectivity.stickers",
                name: "Stickers & Avatars",
                effect: "Stickers, Memoji avatars, and social layer features will not work.",
                memory_mb: 10,
                labels: &[
                    "com.apple.avatarsd",
                    "com.apple.stickersd",
                    "com.apple.sociallayerd",
                ],
            },
            Subgroup {
                id: "connectivity.tvremote",
                name: "Apple TV remote",
                effect: "The Apple TV remote and navigation services will not work.",
                memory_mb: 7,
                labels: &["com.apple.tvremoted", "com.apple.navd"],
            },
        ],
    },
    ServiceGroup {
        id: "telemetry",
        name: "Ads, Diagnostics & Telemetry",
        effect: "DeviceCheck plus analytics, diagnostics, and feedback services are unavailable.",
        kept_by_default: false,
        subgroups: &[
            Subgroup {
                id: "telemetry.ads",
                name: "Ads & Privacy",
                effect: "Ad privacy and promoted content services are unavailable.",
                memory_mb: 20,
                labels: &["com.apple.ap.adprivacyd", "com.apple.ap.promotedcontentd"],
            },
            Subgroup {
                id: "telemetry.diagnostics",
                name: "Diagnostics & Analytics",
                effect: "Diagnostics, analytics, and feedback reporting are unavailable.",
                memory_mb: 45,
                labels: &[
                    "com.apple.diagnosticextensionsd",
                    "com.apple.feedbackd",
                    "com.apple.rtcreportingd",
                    "com.apple.securityuploadd",
                    "com.apple.geoanalyticsd",
                ],
            },
            Subgroup {
                id: "telemetry.experiments",
                name: "Experiments & Follow-ups",
                effect: "Feature experiments and follow-up prompts are unavailable.",
                memory_mb: 25,
                labels: &[
                    "com.apple.triald",
                    "com.apple.followupd",
                    "com.apple.purplebuddy.budd",
                ],
            },
            Subgroup {
                id: "telemetry.devicecheck",
                name: "DeviceCheck",
                effect: "DeviceCheck is unavailable.",
                memory_mb: 15,
                labels: &["com.apple.devicecheckd"],
            },
        ],
    },
    ServiceGroup {
        id: "other",
        name: "Other Background Services",
        effect: "Wallet, merchant, business, asset, and miscellaneous background services are unavailable.",
        kept_by_default: false,
        subgroups: &[Subgroup {
            id: "other.other",
            name: "Other Background Services",
            effect: "Wallet, merchant, business, asset, and miscellaneous background services are unavailable. passd and financed stay on while App Store is kept on.",
            memory_mb: 195,
            labels: &[
                "com.apple.financed",
                "com.apple.passd",
                "com.apple.merchantd",
                "com.apple.coreidvd",
                "com.apple.businessservicesd",
                "com.apple.deviceaccessd",
                "com.apple.replicatord",
                "com.apple.linkd",
                "com.apple.ind",
                "com.apple.storagedatad",
                "com.apple.StatusKitAgent",
                "com.apple.countryd",
                "com.apple.mobileassetd",
                "com.apple.managedconfiguration.passcodenagd",
            ],
        }],
    },
];
