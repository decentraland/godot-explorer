use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures_util::future::try_join_all;
use godot::{
    classes::{AudioStream, Material, Mesh, Os, ResourceLoader, Texture2D},
    obj::Singleton,
    prelude::*,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};

use crate::{
    auth::wallet::AsH160,
    avatars::{dcl_user_profile::DclUserProfile, item::DclItemEntityDefinition},
    content::content_mapping::DclContentMappingAndUrl,
    dcl::common::string::FindNthChar,
    godot_classes::{
        dcl_config::{DclConfig, TextureQuality},
        dcl_global::DclGlobal,
        promise::Promise,
        resource_locker::ResourceLocker,
    },
    http_request::http_queue_requester::HttpQueueRequester,
    scene_runner::tokio_runtime::TokioRuntime,
};

#[cfg(feature = "use_resource_tracking")]
use crate::godot_classes::dcl_resource_tracker::{
    report_download_speed, report_resource_deleted, report_resource_download_done,
    report_resource_downloading, report_resource_error, report_resource_loaded,
    report_resource_start,
};

use super::{
    audio::load_audio,
    gltf::{
        apply_update_set_mask_colliders, load_gltf_emote, load_gltf_scene_content,
        load_gltf_wearable, DclEmoteGltf,
    },
    profile::{prepare_request_requirements, request_lambda_profile},
    resource_provider::ResourceProvider,
    texture::{load_image_texture, TextureEntry},
    thread_safety::{set_thread_safety_checks_enabled, then_promise, GodotSingleThreadSafety},
    video::download_video,
    wearable_entities::{request_wearables, WearableManyResolved},
};

#[cfg(feature = "use_resource_tracking")]
use super::resource_download_tracking::ResourceDownloadTracking;

#[derive(Clone)]
pub struct ContentEntry {
    promise: Gd<Promise>,
    last_access: Instant,
}

pub struct OptimizedData {
    // Set of optimized hashes that we know that exists...
    assets: RwLock<HashSet<String>>,
    // HashMap with all optimized hashes and its dependencies...
    dependencies: RwLock<HashMap<String, HashSet<String>>>,
    // List of optimized assets that were loaded (already added to ProjectSettings.load_resource_pack)
    loaded_assets: RwLock<HashSet<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentData {
    optimized_content: Vec<String>,
    external_scene_dependencies: HashMap<String, HashSet<String>>,
    original_sizes: HashMap<String, ImageSize>,
    hash_size_map: HashMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct ImageSize {
    height: i32,
    width: i32,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct ContentProvider {
    content_folder: Arc<String>,
    resource_provider: Arc<ResourceProvider>,
    #[cfg(feature = "use_resource_tracking")]
    resource_download_tracking: Arc<ResourceDownloadTracking>,
    http_queue_requester: Arc<HttpQueueRequester>,
    cached: HashMap<String, ContentEntry>,
    godot_single_thread: Arc<Semaphore>,
    texture_quality: TextureQuality, // copy from DclGlobal on startup
    every_second_tick: f64,
    download_speed_mbs: f64,
    network_speed_peak_mbs: f64,
    network_download_history_mb: VecDeque<f64>, // Last 60 seconds of download sizes in MB
    loading_resources: Arc<AtomicU64>,
    loaded_resources: Arc<AtomicU64>,
    #[cfg(feature = "use_resource_tracking")]
    tracking_tick: f64,
    optimized_data: Arc<OptimizedData>,
    // Set of optimized hashes that we know that exists...
    optimized_assets: HashSet<String>,
    optimized_original_size: HashMap<String, ImageSize>,
}

#[derive(Clone)]
pub struct ContentProviderContext {
    pub content_folder: Arc<String>,
    pub resource_provider: Arc<ResourceProvider>,
    pub http_queue_requester: Arc<HttpQueueRequester>,
    pub godot_single_thread: Arc<Semaphore>,
    pub texture_quality: TextureQuality, // copy from DclGlobal on startup
}

unsafe impl Send for ContentProviderContext {}

const ASSET_OPTIMIZED_BASE_URL: &str = "https://optimized-assets.dclexplorer.com/v2";

#[godot_api]
impl INode for ContentProvider {
    fn init(_base: Base<Node>) -> Self {
        let content_folder = Arc::new(format!("{}/content/", Os::singleton().get_user_data_dir()));

        #[cfg(feature = "use_resource_tracking")]
        let resource_download_tracking = Arc::new(ResourceDownloadTracking::new());

        Self {
            resource_provider: Arc::new(ResourceProvider::new(
                content_folder.clone().as_str(),
                2048 * 1000 * 1000,
                32,
                #[cfg(feature = "use_resource_tracking")]
                resource_download_tracking.clone(),
            )),
            #[cfg(feature = "use_resource_tracking")]
            resource_download_tracking,
            http_queue_requester: Arc::new(HttpQueueRequester::new(
                6,
                DclGlobal::get_network_inspector_sender(),
            )),
            content_folder,
            cached: HashMap::new(),
            godot_single_thread: Arc::new(Semaphore::new(1)),
            texture_quality: DclConfig::static_get_texture_quality(),
            every_second_tick: 0.0,
            loading_resources: Arc::new(AtomicU64::new(0)),
            loaded_resources: Arc::new(AtomicU64::new(0)),
            download_speed_mbs: 0.0,
            network_speed_peak_mbs: 0.0,
            network_download_history_mb: VecDeque::with_capacity(60),
            #[cfg(feature = "use_resource_tracking")]
            tracking_tick: 0.0,
            optimized_data: Arc::new(OptimizedData {
                assets: RwLock::new(HashSet::default()),
                dependencies: RwLock::new(HashMap::default()),
                loaded_assets: RwLock::new(HashSet::default()),
            }),
            optimized_assets: HashSet::default(),
            optimized_original_size: HashMap::default(),
        }
    }
    fn ready(&mut self) {}
    fn exit_tree(&mut self) {
        self.cached.clear();
        tracing::info!("ContentProvider::exit_tree");
    }

    fn process(&mut self, dt: f64) {
        // Update resource download tracking
        #[cfg(feature = "use_resource_tracking")]
        {
            self.tracking_tick += dt;
            if self.tracking_tick >= 0.1 {
                let mut speed = 0.0;
                self.tracking_tick = 0.0;
                let states = self.resource_download_tracking.consume_downloads_state();
                for (file_hash, state_info) in states {
                    if state_info.done {
                        report_resource_download_done(&file_hash, state_info.current_size);
                    } else {
                        report_resource_downloading(
                            &file_hash,
                            state_info.current_size,
                            state_info.speed,
                        );
                    }
                    speed += state_info.speed;
                }
                report_download_speed(speed);
            }
        }

        self.every_second_tick += dt;
        if self.every_second_tick >= 1.0 {
            self.every_second_tick = 0.0;

            let downloaded_size = self.resource_provider.consume_download_size();
            let downloaded_size_mb = (downloaded_size as f64) / 1024.0 / 1024.0;
            self.download_speed_mbs = downloaded_size_mb;

            // Update peak speed
            if self.download_speed_mbs > self.network_speed_peak_mbs {
                self.network_speed_peak_mbs = self.download_speed_mbs;
            }

            // Track last 60 seconds of downloads
            self.network_download_history_mb
                .push_back(downloaded_size_mb);
            if self.network_download_history_mb.len() > 60 {
                self.network_download_history_mb.pop_front();
            }

            // Clean cache
            self.cached.retain(|_hash_id, entry| {
                // don't add a timeout for promise to be resolved,
                // that timeout should be done on the fetch process
                // resolved doesn't mean that is resolved correctly
                let process_promise = entry.last_access.elapsed() > Duration::from_secs(30)
                    && entry.promise.bind().is_resolved();
                if process_promise {
                    let data = entry.promise.bind().get_data();
                    if let Ok(mut node_3d) = Gd::<Node3D>::try_from_variant(&data) {
                        if let Some(resource_locker) =
                            node_3d.get_node_or_null(&NodePath::from("ResourceLocker"))
                        {
                            if let Ok(resource_locker) =
                                resource_locker.try_cast::<ResourceLocker>()
                            {
                                let reference_count = resource_locker.bind().get_reference_count();
                                if reference_count == 1 {
                                    #[cfg(feature = "use_resource_tracking")]
                                    report_resource_deleted(&_hash_id);
                                    node_3d.queue_free();
                                    return false;
                                }
                            }
                        }
                    } else if let Ok(ref_counted) = Gd::<RefCounted>::try_from_variant(&data) {
                        let reference_count = ref_counted.get_reference_count();
                        if reference_count == 1 {
                            return false;
                        }
                    }
                }
                true
            });
        }
    }
}

#[godot_api]
impl ContentProvider {
    #[func]
    pub fn fetch_optimized_asset_with_dependencies(&mut self, file_hash: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let ctx = self.get_context();
        let optimized_data = self.optimized_data.clone();

        let file_hash = file_hash.to_string();
        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();

        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&file_hash);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let _ =
                ContentProvider::async_fetch_optimized_asset(file_hash, ctx, optimized_data, true)
                    .await;

            then_promise(get_promise, Ok(None));

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        promise
    }

    #[func]
    pub fn fetch_optimized_asset(&mut self, file_hash: GString) -> Gd<Promise> {
        if self.optimized_asset_exists(file_hash.clone()) {
            return Promise::from_rejected(format!(
                "Optimized asset hash={} doesn't exists",
                file_hash
            ));
        }

        let (promise, get_promise) = Promise::make_to_async();
        let ctx = self.get_context();
        let optimized_data = self.optimized_data.clone();

        let file_hash = file_hash.to_string();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&file_hash);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let _ =
                ContentProvider::async_fetch_optimized_asset(file_hash, ctx, optimized_data, false)
                    .await;
            then_promise(get_promise, Ok(None));

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        promise
    }

    #[func]
    pub fn optimized_asset_exists(&self, file_hash: GString) -> bool {
        self.optimized_assets.contains(&file_hash.to_string())
    }

    #[func]
    pub fn load_optimized_assets_metadata(&mut self, file_content: GString) -> Gd<Promise> {
        let content_data: Result<ContentData, serde_json::Error> =
            serde_json::from_str(&file_content.to_string());

        let (promise, get_promise) = Promise::make_to_async();

        if let Ok(content_data) = content_data {
            self.optimized_original_size
                .extend(content_data.original_sizes);

            self.optimized_assets
                .extend(content_data.optimized_content.clone());

            let optimized_data = self.optimized_data.clone();

            TokioRuntime::spawn(async move {
                optimized_data
                    .dependencies
                    .write()
                    .await
                    .extend(content_data.external_scene_dependencies);
                optimized_data
                    .assets
                    .write()
                    .await
                    .extend(content_data.optimized_content);
                then_promise(get_promise, Ok(None));
            });
        } else if let Err(error) = content_data {
            //promise.bind_mut().reject();
            then_promise(
                get_promise,
                Err(anyhow::anyhow!(format!(
                    "Failed to parse content data of the scene {:?}",
                    error
                ))),
            );
        }

        promise
    }

    #[func]
    pub fn fetch_wearable_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        let cache_key = format!("wearable:{}", file_hash);
        if let Some(entry) = self.cached.get_mut(&cache_key) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let (promise, get_promise) = Promise::make_to_async();
        let gltf_file_path = file_path.to_string();
        let content_provider_context = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.to_string();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result =
                load_gltf_wearable(gltf_file_path, content_mapping, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            cache_key,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_scene_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        if let Some(entry) = self.cached.get_mut(file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let file_hash = file_hash.clone();
        let (promise, get_promise) = Promise::make_to_async();
        let gltf_file_path = file_path.to_string();
        let content_provider_context = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result =
                load_gltf_scene_content(gltf_file_path, content_mapping, content_provider_context)
                    .await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_emote_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        let cache_key = format!("emote:{}", file_hash);
        if let Some(entry) = self.cached.get_mut(&cache_key) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let (promise, get_promise) = Promise::make_to_async();
        let gltf_file_path = file_path.to_string();
        let content_provider_context = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.to_string();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result =
                load_gltf_emote(gltf_file_path, content_mapping, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            cache_key,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn instance_gltf_colliders(
        &mut self,
        gltf_node: Gd<Node>,
        dcl_visible_cmask: i32,
        dcl_invisible_cmask: i32,
        dcl_scene_id: i32,
        dcl_entity_id: i32,
    ) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let gltf_node_instance_id = gltf_node.instance_id();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            let result = apply_update_set_mask_colliders(
                gltf_node_instance_id,
                dcl_visible_cmask,
                dcl_invisible_cmask,
                dcl_scene_id,
                dcl_entity_id,
                content_provider_context,
            )
            .await;
            then_promise(get_promise, result);
        });

        promise
    }

    #[func]
    pub fn fetch_file(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = content_mapping.bind().get_hash(file_path);
        let url = format!("{}{}", content_mapping.bind().get_base_url(), file_hash);

        self.fetch_file_by_url(file_hash, GString::from(url.as_str()))
    }

    #[func]
    pub fn fetch_file_by_url(&mut self, file_hash: GString, url: GString) -> Gd<Promise> {
        let file_hash = file_hash.to_string();

        // Check cache first - prevent duplicate downloads of the same file
        if let Some(entry) = self.cached.get_mut(&file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let url = url.to_string();
        let (promise, get_promise) = Promise::make_to_async();
        let ctx = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let absolute_file_path = format!("{}{}", ctx.content_folder, hash_id);

            if ctx
                .resource_provider
                .fetch_resource(url, hash_id.clone(), absolute_file_path)
                .await
                .is_ok()
            {
                #[cfg(feature = "use_resource_tracking")]
                report_resource_loaded(&hash_id.clone());

                then_promise(get_promise, Ok(None));
            } else {
                let error = anyhow::anyhow!("Failed to download file");

                #[cfg(feature = "use_resource_tracking")]
                report_resource_error(&hash_id.clone(), &error.to_string());

                then_promise(get_promise, Err(error));
            }
            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        // Insert into cache to prevent duplicate downloads
        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn store_file(&mut self, file_hash: GString, bytes: PackedByteArray) -> Gd<Promise> {
        let file_hash = file_hash.to_string();

        let (promise, get_promise) = Promise::make_to_async();
        let ctx = self.get_context();

        let bytes = bytes.to_vec();

        TokioRuntime::spawn(async move {
            if ctx
                .resource_provider
                .store_file(file_hash.as_str(), bytes.as_slice())
                .await
                .is_ok()
            {
                then_promise(get_promise, Ok(None));
            } else {
                then_promise(get_promise, Err(anyhow::anyhow!("Failed to store file")));
            }
        });

        promise
    }

    #[func]
    pub fn fetch_audio(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        if let Some(entry) = self.cached.get_mut(file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let file_hash = file_hash.clone();
        let (promise, get_promise) = Promise::make_to_async();
        let audio_file_path = file_path.to_string();
        let content_provider_context = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result =
                load_audio(audio_file_path, content_mapping, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }
            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_texture(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = content_mapping.bind().get_hash(file_path);
        if file_hash.is_empty() {
            return Promise::from_rejected("Texture not found in the mappings.".to_string());
        };

        self.fetch_texture_by_hash(file_hash, content_mapping)
    }

    #[func]
    pub fn fetch_texture_by_hash(
        &mut self,
        file_hash_godot: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = file_hash_godot.to_string();
        if let Some(entry) = self.cached.get_mut(&file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        // TODO: In the future, this would be handled by each component handler
        //  and check if the hostname is allowed (set up in the scene.json)
        //  https://github.com/decentraland/godot-explorer/issues/363
        if file_hash.starts_with("http") {
            // get file_hash from url
            let new_file_hash = format!("hashed_{:x}", file_hash_godot.hash_u32());
            let promise = self.fetch_texture_by_url(GString::from(&new_file_hash), file_hash_godot);
            self.cached.insert(
                file_hash,
                ContentEntry {
                    last_access: Instant::now(),
                    promise: promise.clone(),
                },
            );
            return promise;
        }

        let (promise, get_promise) = Promise::make_to_async();
        let ctx = self.get_context();

        if self.optimized_asset_exists(file_hash_godot.clone()) {
            let hash_id = file_hash.clone();
            let optimized_data = self.optimized_data.clone();

            let original_size = self.optimized_original_size.get(&hash_id).copied();

            TokioRuntime::spawn(async move {
                let _ = ContentProvider::async_fetch_optimized_asset(
                    hash_id.clone(),
                    ctx,
                    optimized_data,
                    false,
                )
                .await;

                let godot_path = format!("res://content/{}", hash_id);

                let resource = ResourceLoader::singleton()
                    .load(&GString::from(godot_path.as_str()))
                    .unwrap();

                let texture = resource.cast::<godot::classes::Texture2D>();
                let image = texture.get_image().unwrap();

                let original_size = if let Some(original_size) = original_size {
                    Vector2i::new(original_size.width, original_size.height)
                } else {
                    image.get_size()
                };

                let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
                    original_size,
                    image,
                    texture,
                });

                then_promise(get_promise, Ok(Some(texture_entry.to_variant())));
            });
        } else {
            let url = format!(
                "{}{}",
                content_mapping.bind().get_base_url(),
                file_hash.clone()
            );

            let loading_resources = self.loading_resources.clone();
            let loaded_resources = self.loaded_resources.clone();
            let hash_id = file_hash.clone();
            TokioRuntime::spawn(async move {
                #[cfg(feature = "use_resource_tracking")]
                report_resource_start(&hash_id);

                loading_resources.fetch_add(1, Ordering::Relaxed);

                let result = load_image_texture(url, hash_id.clone(), ctx).await;

                #[cfg(feature = "use_resource_tracking")]
                if let Err(error) = &result {
                    report_resource_error(&hash_id, &error.to_string());
                } else {
                    report_resource_loaded(&hash_id);
                }

                then_promise(get_promise, result);

                loaded_resources.fetch_add(1, Ordering::Relaxed);
            });
        }

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    /// Fetches a texture by hash, bypassing the optimization pipeline.
    /// This is useful for UI textures that need the original quality.
    /// Uses a separate cache key (`{hash}_original`) to avoid conflicts with optimized versions.
    pub fn fetch_texture_by_hash_original(
        &mut self,
        file_hash_godot: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = file_hash_godot.to_string();
        let cache_key = format!("{}_original", file_hash);

        if let Some(entry) = self.cached.get_mut(&cache_key) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        // Handle URL-based textures
        if file_hash.starts_with("http") {
            let new_file_hash = format!("hashed_{:x}_original", file_hash_godot.hash_u32());
            let promise =
                self.fetch_texture_by_url_original(GString::from(&new_file_hash), file_hash_godot);
            self.cached.insert(
                cache_key,
                ContentEntry {
                    last_access: Instant::now(),
                    promise: promise.clone(),
                },
            );
            return promise;
        }

        let (promise, get_promise) = Promise::make_to_async();

        // Create context with Source quality to bypass resize optimization
        let mut ctx = self.get_context();
        ctx.texture_quality = TextureQuality::Source;

        let url = format!(
            "{}{}",
            content_mapping.bind().get_base_url(),
            file_hash.clone()
        );

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        let hash_id = file_hash.clone();

        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result = load_image_texture(url, hash_id.clone(), ctx).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            cache_key,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    /// Fetches a texture by URL, bypassing the optimization pipeline.
    /// Uses Source quality to preserve original texture resolution.
    pub fn fetch_texture_by_url_original(
        &mut self,
        file_hash: GString,
        url: GString,
    ) -> Gd<Promise> {
        let file_hash = file_hash.to_string();
        if let Some(entry) = self.cached.get_mut(&file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }
        let url = url.to_string();
        let (promise, get_promise) = Promise::make_to_async();

        // Create context with Source quality to bypass resize optimization
        let mut content_provider_context = self.get_context();
        content_provider_context.texture_quality = TextureQuality::Source;

        let sent_file_hash = file_hash.clone();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();

        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result = load_image_texture(url, sent_file_hash, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_texture_by_url(&mut self, file_hash: GString, url: GString) -> Gd<Promise> {
        let file_hash = file_hash.to_string();
        if let Some(entry) = self.cached.get_mut(&file_hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }
        let url = url.to_string();
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();
        let sent_file_hash = file_hash.clone();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();

        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result = load_image_texture(url, sent_file_hash, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn get_texture_from_hash(&mut self, file_hash: GString) -> Option<Gd<Texture2D>> {
        let entry = self.cached.get_mut(&file_hash.to_string())?;
        entry.last_access = Instant::now();
        let promise_data = entry.promise.bind().get_data();
        let texture_entry = promise_data.try_to::<Gd<TextureEntry>>().ok()?;
        let texture = texture_entry.bind().texture.clone();
        Some(texture)
    }

    #[func]
    pub fn get_gltf_from_hash(&mut self, file_hash: GString) -> Option<Gd<Node3D>> {
        let cache_key = format!("wearable:{}", file_hash);
        let entry = self.cached.get_mut(&cache_key)?;
        entry.last_access = Instant::now();
        entry.promise.bind().get_data().try_to::<Gd<Node3D>>().ok()
    }

    #[func]
    pub fn get_emote_gltf_from_hash(&mut self, file_hash: GString) -> Option<Gd<DclEmoteGltf>> {
        let cache_key = format!("emote:{}", file_hash);
        let entry = self.cached.get_mut(&cache_key)?;
        entry.last_access = Instant::now();
        entry
            .promise
            .bind()
            .get_data()
            .try_to::<Gd<DclEmoteGltf>>()
            .ok()
    }

    #[func]
    pub fn get_audio_from_hash(&mut self, file_hash: GString) -> Option<Gd<AudioStream>> {
        let entry = self.cached.get_mut(&file_hash.to_string())?;
        entry.last_access = Instant::now();
        entry
            .promise
            .bind()
            .get_data()
            .try_to::<Gd<AudioStream>>()
            .ok()
    }

    #[func]
    pub fn is_resource_from_hash_loaded(&self, file_hash: GString) -> bool {
        if let Some(entry) = self.cached.get(&file_hash.to_string()) {
            return entry.promise.bind().is_resolved();
        }
        false
    }

    #[func]
    pub fn fetch_video(
        &mut self,
        file_hash: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let (promise, get_promise) = Promise::make_to_async();
        let file_hash = file_hash.to_string();
        let video_file_hash = file_hash.clone();
        let content_provider_context = self.get_context();

        let loading_resources = self.loading_resources.clone();
        let loaded_resources = self.loaded_resources.clone();
        #[cfg(feature = "use_resource_tracking")]
        let hash_id = file_hash.clone();
        TokioRuntime::spawn(async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&hash_id);

            loading_resources.fetch_add(1, Ordering::Relaxed);

            let result =
                download_video(video_file_hash, content_mapping, content_provider_context).await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(error) = &result {
                report_resource_error(&hash_id, &error.to_string());
            } else {
                report_resource_loaded(&hash_id);
            }

            then_promise(get_promise, result);

            loaded_resources.fetch_add(1, Ordering::Relaxed);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn duplicate_materials(&mut self, target_meshes: VarArray) -> Gd<Promise> {
        let data = target_meshes
            .iter_shared()
            .map(|dict| {
                let dict = dict.try_to::<VarDictionary>().ok()?;
                let mesh = dict.get("mesh")?.try_to::<Gd<Mesh>>().ok()?;
                let n = dict.get("n")?.try_to::<i32>().ok()?;

                Some((mesh.instance_id(), n))
            })
            .filter(|v| v.is_some())
            .flatten()
            .collect::<Vec<_>>();

        let (promise, get_promise) = Promise::make_to_async();
        TokioRuntime::spawn(async move {
            set_thread_safety_checks_enabled(false);

            for (mesh_instance_id, n) in data {
                let mut mesh = Gd::<Mesh>::from_instance_id(mesh_instance_id);
                for i in 0..n {
                    let Some(new_material) = mesh.surface_get_material(i) else {
                        continue;
                    };
                    let Some(new_material) = new_material.duplicate() else {
                        continue;
                    };

                    mesh.surface_set_material(i, &new_material.cast::<Material>());
                }
            }

            set_thread_safety_checks_enabled(true);

            then_promise(get_promise, Ok(None));
        });

        promise
    }

    #[func]
    pub fn fetch_wearables(
        &mut self,
        wearables: VarArray,
        content_base_url: GString,
    ) -> Array<Gd<Promise>> {
        let mut promise_ids = HashSet::new();
        let mut new_promise = None;
        let mut wearable_to_fetch = HashSet::new();

        for wearable in wearables.iter_shared() {
            let wearable_id = wearable.to_string();
            let token_id_pos = wearable_id
                .find_nth_char(6, ':')
                .unwrap_or(wearable_id.len());
            let wearable_id = wearable_id[0..token_id_pos].to_lowercase();

            if let Some(entry) = self.cached.get_mut(&wearable_id) {
                entry.last_access = Instant::now();
                promise_ids.insert(entry.promise.instance_id());
            } else {
                wearable_to_fetch.insert(wearable_id.clone());
                if new_promise.is_none() {
                    let (promise, get_promise) = Promise::make_to_async();
                    promise_ids.insert(promise.instance_id());
                    new_promise = Some((promise, get_promise));
                }

                self.cached.insert(
                    wearable_id,
                    ContentEntry {
                        last_access: Instant::now(),
                        promise: new_promise.as_ref().unwrap().0.clone(),
                    },
                );
            }
        }

        if !wearable_to_fetch.is_empty() {
            let (promise, get_promise) = new_promise.unwrap();
            let content_provider_context = self.get_context();
            let content_base_url = content_base_url.to_string();
            let extra_slash = if content_base_url.ends_with('/') {
                ""
            } else {
                "/"
            };
            let content_base_url = format!("{}{extra_slash}", content_base_url);
            let ipfs_content_base_url = format!("{content_base_url}contents/");
            TokioRuntime::spawn(async move {
                let result = request_wearables(
                    content_base_url,
                    ipfs_content_base_url,
                    wearable_to_fetch.into_iter().collect(),
                    content_provider_context,
                )
                .await;
                then_promise(get_promise, result);
            });
            self.cached.insert(
                "wearables".to_string(),
                ContentEntry {
                    last_access: Instant::now(),
                    promise: promise.clone(),
                },
            );
        }

        Array::from_iter(promise_ids.into_iter().map(Gd::from_instance_id))
    }

    #[func]
    pub fn get_wearable(&mut self, id: GString) -> Option<Gd<DclItemEntityDefinition>> {
        let id = id.to_string();
        let token_id_pos = id.find_nth_char(6, ':').unwrap_or(id.len());
        let id = id[0..token_id_pos].to_lowercase();

        if let Some(entry) = self.cached.get_mut(&id) {
            entry.last_access = Instant::now();
            if let Ok(results) = entry
                .promise
                .bind()
                .get_data()
                .try_to::<Gd<WearableManyResolved>>()
            {
                if let Some(wearable) = results.bind().wearable_map.get(&id) {
                    return Some(DclItemEntityDefinition::from_gd(wearable.clone()));
                }
            }
        }
        None
    }

    #[func]
    pub fn get_pending_promises(&self) -> Array<Gd<Promise>> {
        Array::from_iter(
            self.cached
                .iter()
                .filter(|(_, entry)| !entry.promise.bind().is_resolved())
                .map(|(_, entry)| entry.promise.clone()),
        )
    }

    #[func]
    pub fn get_profile(&mut self, user_id: GString) -> Option<Gd<DclUserProfile>> {
        let user_id = user_id.to_string().as_str().as_h160()?;
        let hash = format!("profile_{:x}", user_id);
        if let Some(entry) = self.cached.get_mut(&hash) {
            entry.last_access = Instant::now();
            let promise_data = entry.promise.bind().get_data();
            promise_data.try_to::<Gd<DclUserProfile>>().ok()
        } else {
            None
        }
    }

    #[func]
    pub fn clear_cache_folder(&self) {
        let resource_provider = self.resource_provider.clone();
        TokioRuntime::spawn(async move {
            resource_provider.clear().await;
        });
    }

    #[func]
    pub fn set_cache_folder_max_size(&mut self, size: i64) {
        self.resource_provider.set_max_cache_size(size)
    }

    #[func]
    pub fn get_cache_folder_total_size(&mut self) -> i64 {
        self.resource_provider.get_cache_total_size()
    }

    #[func]
    pub fn get_download_speed_mbs(&self) -> f64 {
        self.download_speed_mbs
    }

    #[func]
    pub fn get_network_speed_peak_mbs(&self) -> f64 {
        self.network_speed_peak_mbs
    }

    #[func]
    pub fn get_network_used_last_minute_mb(&self) -> f64 {
        self.network_download_history_mb.iter().sum()
    }

    #[func]
    pub fn count_loaded_resources(&self) -> u64 {
        self.loaded_resources.load(Ordering::Relaxed)
    }

    #[func]
    pub fn count_loading_resources(&self) -> u64 {
        self.loading_resources.load(Ordering::Relaxed)
    }

    #[func]
    pub fn set_max_concurrent_downloads(&mut self, number: i32) {
        self.resource_provider
            .set_max_concurrent_downloads(number as usize)
    }

    #[func]
    pub fn get_optimized_base_url(&self) -> GString {
        ASSET_OPTIMIZED_BASE_URL.to_godot()
    }

    #[func]
    pub fn fetch_profile(&mut self, user_id: GString) -> Gd<Promise> {
        let Some(user_id) = user_id.to_string().as_str().as_h160() else {
            return Promise::from_rejected("Invalid user id".to_string());
        };

        let hash = format!("profile_{:x}", user_id);
        if let Some(entry) = self.cached.get_mut(&hash) {
            entry.last_access = Instant::now();
            return entry.promise.clone();
        }

        let (lamda_server_base_url, profile_base_url, http_requester) =
            prepare_request_requirements();
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();

        TokioRuntime::spawn(async move {
            let result = request_lambda_profile(
                user_id,
                lamda_server_base_url.as_str(),
                profile_base_url.as_str(),
                http_requester,
            )
            .await;

            let Some(_thread_safe_check) =
                GodotSingleThreadSafety::acquire_owned(&content_provider_context).await
            else {
                tracing::error!("Failed to acquire semaphore");
                return;
            };
            let result = result.map(|value| Some(DclUserProfile::from_gd(value).to_variant()));

            then_promise(get_promise, result)
        });

        self.cached.insert(
            hash,
            ContentEntry {
                last_access: Instant::now(),
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn purge_file(&mut self, file_hash: GString) -> Gd<Promise> {
        let file_hash_str = file_hash.to_string();
        let absolute_file_path = format!("{}{}", self.content_folder, file_hash);

        let resource_provider = self.resource_provider.clone();
        let (promise, get_promise) = Promise::make_to_async();

        self.cached.remove(&file_hash_str);

        TokioRuntime::spawn(async move {
            resource_provider.delete_file(&absolute_file_path).await;
            then_promise(get_promise, Ok(None));
        });

        promise
    }
}

impl ContentProvider {
    fn get_context(&self) -> ContentProviderContext {
        ContentProviderContext {
            content_folder: self.content_folder.clone(),
            http_queue_requester: self.http_queue_requester.clone(),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
        }
    }

    pub async fn async_fetch_optimized_asset(
        file_hash: String,
        ctx: ContentProviderContext,
        optimized_data: Arc<OptimizedData>,
        with_dependencies: bool,
    ) -> Result<(), String> {
        // 1. We search which dependencies we need to download
        let mut futures_to_wait: Vec<_> = Vec::default();
        let mut hashes_to_load: Vec<String> = Vec::default();

        let dependencies = {
            if with_dependencies {
                let dependencies_guard = optimized_data.dependencies.read().await;
                let mut deps = dependencies_guard
                    .get(&file_hash)
                    .cloned()
                    .unwrap_or_default();

                deps.insert(file_hash.clone());
                deps // Return the modified set
            } else {
                HashSet::from([file_hash.clone()])
            }
        };

        let loaded_dependencies = optimized_data.loaded_assets.read().await;

        for hash_dependency in &dependencies {
            let asset_url: String = format!(
                "{}/{}-mobile.zip",
                ASSET_OPTIMIZED_BASE_URL, hash_dependency
            );
            let hash_dependency_zip = format!("{}-mobile.zip", hash_dependency);
            let absolute_file_path = format!("{}{}", ctx.content_folder, hash_dependency_zip);

            if !loaded_dependencies.contains(hash_dependency) {
                if hash_dependency != &file_hash {
                    // we don't add the own file
                    hashes_to_load.push(hash_dependency.clone());
                }
            } else if ctx
                .resource_provider
                .file_exists(&hash_dependency_zip)
                .await
            {
                continue; // Skip fetching if the dependency exists in cache
            }

            // Fetch the resource if it's either a new dependency or missing in cache

            let future = ctx.resource_provider.fetch_resource(
                asset_url.clone(),
                hash_dependency_zip.clone(),
                absolute_file_path,
            );
            futures_to_wait.push(future);
        }

        // 1.1 We ensure that the file_hash (the scene who is requesting) is the last dependency to load
        hashes_to_load.push(file_hash);

        // 2. We add what we are going to load into the loaded_dependencies
        drop(loaded_dependencies); // drop read, before writing
        let mut loaded_dependencies = optimized_data.loaded_assets.write().await;
        for hash_to_load in &hashes_to_load {
            loaded_dependencies.insert(hash_to_load.clone());
        }
        drop(loaded_dependencies); // drop write

        // 3. Wait all downloads
        let _ = try_join_all(futures_to_wait).await;

        // 4. Load what was listed
        for hash_to_load in &hashes_to_load {
            let hash_zip = format!("{}-mobile.zip", hash_to_load);
            let zip_path = &format!("user://content/{}", hash_zip);
            let result = godot::classes::ProjectSettings::singleton()
                .load_resource_pack_ex(zip_path)
                .replace_files(false)
                .done();

            if !result {
                godot_error!("load_resource_pack failed on {zip_path}");
            }
        }

        Ok(())
    }
}
