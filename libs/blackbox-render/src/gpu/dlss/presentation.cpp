// Included by bridge.cpp: Reflex/PCL and DLSS-G use exactly the SR frame token and viewport.
// API reference: Streamline 2.14.16 ProgrammingGuideReflex/PCL/DLSS_G (MIT).
static PFun_slReflexSetOptions* reflex_options{};
static PFun_slReflexSleep* reflex_sleep{};
static PFun_slPCLSetMarker* pcl_marker{};
static PFun_slDLSSGSetOptions* generation_options{};
static PFun_slDLSSGGetState* generation_state{};
static bool render_started{}, generation_on{}, reflex_ready{};
static uint32_t generation_max{}, generation_min{}, presented{}, generation_error{};
static uint64_t marker_counts[8]{};

template<class T> static bool feature_function(sl::Feature feature, const char* name, T*& function) {
    return p_slGetFeatureFunction(feature, name, reinterpret_cast<void*&>(function)) == sl::Result::eOk && function;
}

extern "C" void bb_sl_latency_caps(void* physical, uint32_t* caps) {
    sl::AdapterInfo info{};
    info.vkPhysicalDevice = physical;
    if (p_slIsFeatureSupported(sl::kFeatureReflex, info) == sl::Result::eOk &&
        p_slIsFeatureSupported(sl::kFeaturePCL, info) == sl::Result::eOk) {
        PFun_slReflexGetState* get_state{};
        PFun_slPCLSetOptions* pcl_options{};
        sl::ReflexState state{};
        if (feature_function(sl::kFeatureReflex, "slReflexGetState", get_state) &&
            feature_function(sl::kFeatureReflex, "slReflexSetOptions", reflex_options) &&
            feature_function(sl::kFeatureReflex, "slReflexSleep", reflex_sleep) &&
            feature_function(sl::kFeaturePCL, "slPCLSetMarker", pcl_marker) &&
            feature_function(sl::kFeaturePCL, "slPCLSetOptions", pcl_options) &&
            get_state(state) == sl::Result::eOk) {
            caps[0] = state.lowLatencyAvailable;
            reflex_ready = true;
            sl::PCLOptions options{};
            // The SDK handles the verification hotkey; no window-procedure subclass is installed.
            options.virtualKey = sl::PCLHotKey::eVK_F13;
            pcl_options(options);
        }
    }
    if (caps[0] && p_slIsFeatureSupported(sl::kFeatureDLSS_G, info) == sl::Result::eOk &&
        feature_function(sl::kFeatureDLSS_G, "slDLSSGSetOptions", generation_options) &&
        feature_function(sl::kFeatureDLSS_G, "slDLSSGGetState", generation_state)) {
        sl::DLSSGState state{};
        if (generation_state(viewport, state, nullptr) == sl::Result::eOk) {
            generation_max = state.numFramesToGenerateMax;
            generation_min = state.minWidthOrHeight;
        }
    }
    caps[1] = generation_max;
    caps[2] = generation_min;
    retain_plugin_code();
}

extern "C" int bb_sl_marker(uint32_t marker) {
    if (!current_frame || !pcl_marker) return -1;
    auto result = pcl_marker(static_cast<sl::PCLMarker>(marker), *current_frame);
    if (result == sl::Result::eOk && marker < 8) ++marker_counts[marker];
    return static_cast<int>(result);
}

extern "C" int bb_sl_begin(uint32_t mode, uint32_t limit_us) {
    if (current_frame) return 0;
    if (p_slGetNewFrameToken(current_frame, nullptr) != sl::Result::eOk) return -1;
    render_started = false;
    constants_set = false;
    int result = -1;
    if (reflex_ready) {
        sl::ReflexOptions options{};
        options.mode = static_cast<sl::ReflexMode>(mode);
        options.frameLimitUs = limit_us;
        auto set = reflex_options(options);
        auto sleep = reflex_sleep(*current_frame);
        if (set == sl::Result::eOk && sleep == sl::Result::eOk) result = 0;
    }
    bb_sl_marker(0);
    return result;
}

extern "C" void bb_sl_render_start() {
    if (render_started || !current_frame) return;
    bb_sl_marker(1);
    bb_sl_marker(2);
    render_started = true;
}

extern "C" void bb_sl_end() {
    current_frame = nullptr;
    render_started = false;
}

extern "C" int bb_sl_generation(uint32_t count, uint32_t width, uint32_t height) {
    if (!generation_options) return count ? -1 : 0;
    sl::DLSSGOptions options{};
    options.mode = count ? sl::DLSSGMode::eOn : sl::DLSSGMode::eOff;
    options.numFramesToGenerate = count ? count : 1;
    // One presenting queue: SL blocks it until inputs are consumed. Do not enable unblocked queues.
    options.queueParallelismMode = sl::DLSSGQueueParallelismMode::eBlockPresentingClientQueue;
    options.flags = sl::DLSSGFlags::eRetainResourcesWhenOff;
    options.enableUserInterfaceRecomposition = sl::Boolean::eTrue;
    options.colorWidth = width;
    options.colorHeight = height;
    auto result = generation_options(viewport, options);
    generation_on = count && result == sl::Result::eOk;
    return static_cast<int>(result);
}

extern "C" void bb_sl_suspend() {
    if (!generation_options) return;
    bb_sl_generation(0, 0, 0);
    p_slFreeResources(sl::kFeatureDLSS_G, viewport);
}

extern "C" int bb_sl_tag_generation(const Image* images, const Frame* frame) {
    if (!generation_on || !current_frame) return -1;
    auto result = common_constants(images, frame);
    if (result != sl::Result::eOk) return static_cast<int>(result);
    sl::Resource resources[4];
    sl::BufferType types[] = {sl::kBufferTypeDepth, sl::kBufferTypeMotionVectors, sl::kBufferTypeHUDLessColor, sl::kBufferTypeUIAlpha};
    sl::ResourceTag tags[4];
    for (uint32_t i = 0; i < 4; ++i) {
        const auto& image = images[i];
        auto& resource = resources[i];
        resource.native = reinterpret_cast<void*>(image.image);
        resource.view = reinterpret_cast<void*>(image.view);
        resource.width = image.width;
        resource.height = image.height;
        resource.nativeFormat = image.format;
        resource.state = image.layout;
        resource.usage = image.usage;
        resource.mipLevels = 1;
        resource.arrayLayers = 1;
        tags[i] = sl::ResourceTag(&resource, types[i], sl::ResourceLifecycle::eValidUntilPresent);
        tags[i].extent = {0, 0, image.width, image.height};
    }
    return static_cast<int>(p_slSetTagForFrame(*current_frame, viewport, tags, 4, nullptr));
}

extern "C" int bb_sl_presented(uint32_t* stats) {
    if (!generation_state) return -1;
    sl::DLSSGState state{};
    auto result = generation_state(viewport, state, nullptr);
    if (result == sl::Result::eOk) {
        presented = state.numFramesActuallyPresented;
        generation_error = static_cast<uint32_t>(state.status);
    }
    stats[0] = presented;
    stats[1] = generation_error;
    return static_cast<int>(result);
}

extern "C" uint64_t bb_sl_markers(uint32_t marker) { return marker < 8 ? marker_counts[marker] : 0; }
