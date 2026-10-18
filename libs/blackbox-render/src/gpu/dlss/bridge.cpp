// Streamline's MIT headers define the C++ ABI; Rust sees only this small C interface.
// Integration reference: NVIDIA Streamline 2.14.16 ProgrammingGuideDLSS.md.
#include <windows.h>
#include <tlhelp32.h>
#include <cstring>
#include <cwchar>
#include <limits>
#include <sl.h>
#include <wintrust.h>
#include <sl_security.h>

#define BB_DECL(name) PFun_##name* p_##name{}
BB_DECL(slInit);
BB_DECL(slShutdown);
BB_DECL(slIsFeatureSupported);
BB_DECL(slGetFeatureFunction);
BB_DECL(slSetConstants);
BB_DECL(slGetNewFrameToken);
BB_DECL(slSetTagForFrame);
BB_DECL(slEvaluateFeature);
BB_DECL(slFreeResources);
#include <sl_dlss.h>
#include <sl_dlss_g.h>
#include <sl_reflex.h>
#include <sl_pcl.h>
static_assert(SL_VERSION_MAJOR == 2 && SL_VERSION_MINOR == 14 && SL_VERSION_PATCH == 16,
    "This bridge is validated against Streamline 2.14.16; review ABI changes before upgrading");

static HMODULE module;
static bool initialized;
static uint64_t evaluations;
static sl::ViewportHandle viewport(0);
static sl::FrameToken* current_frame{};
static bool constants_set{};
extern "C" int bb_sl_begin(uint32_t mode, uint32_t limit_us);
extern "C" void bb_sl_end();
extern "C" void bb_sl_suspend();

static void retain_plugin_code() {
    // SDK plugins can register CRT thread-exit callbacks. Keep their code mapped after slShutdown,
    // including NVIDIA App's override plugins, until Windows has finished process/thread teardown.
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, GetCurrentProcessId());
    if (snapshot == INVALID_HANDLE_VALUE) return;
    MODULEENTRY32W entry{};
    entry.dwSize = sizeof(entry);
    if (Module32FirstW(snapshot, &entry)) {
        do {
            // DLSS-G's static caches own common compute resources. Let slShutdown unload it
            // in dependency order, before sl.common tears down the compute interface.
            if (wcscmp(entry.szModule, L"sl.dlss_g.dll") == 0
                || wcsstr(entry.szExePath, L"sl_dlss_g_override_")) continue;
            bool plugin = wcsncmp(entry.szModule, L"sl.", 3) == 0
                || wcsstr(entry.szExePath, L"\\sl_common_override_")
                || wcsstr(entry.szExePath, L"\\sl_dlss_override_")
                || wcsstr(entry.szExePath, L"\\sl_pcl_override_")
                || wcsstr(entry.szExePath, L"\\sl_reflex_override_");
            if (!plugin) continue;
            HMODULE retained{};
            GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
                reinterpret_cast<LPCWSTR>(entry.modBaseAddr), &retained);
        } while (Module32NextW(snapshot, &entry));
    }
    CloseHandle(snapshot);
}

extern "C" int bb_sl_init(const wchar_t* directory, const wchar_t* logs) {
    if (initialized) return -1;
    const auto length = wcslen(directory);
    if (length > 32740) return -5;
    wchar_t dll[32768]{};
    memcpy(dll, directory, length * sizeof(wchar_t));
    wcscpy(dll + length, L"\\sl.interposer.dll");
    if (!sl::security::verifyEmbeddedSignature(dll)) return -2;
    module = LoadLibraryExW(dll, nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
    if (!module) return -3;
#define IMPORT(name) p_##name = reinterpret_cast<PFun_##name*>(GetProcAddress(module, #name)); if (!p_##name) return -4
    IMPORT(slInit);
    IMPORT(slShutdown);
    IMPORT(slIsFeatureSupported);
    IMPORT(slGetFeatureFunction);
    IMPORT(slSetConstants);
    IMPORT(slGetNewFrameToken);
    IMPORT(slSetTagForFrame);
    IMPORT(slEvaluateFeature);
    IMPORT(slFreeResources);
#undef IMPORT
    sl::Preferences preferences{};
    sl::Feature features[] = {sl::kFeatureDLSS, sl::kFeatureReflex, sl::kFeaturePCL, sl::kFeatureDLSS_G};
    const wchar_t* paths[] = {directory};
    preferences.pathsToPlugins = paths;
    preferences.numPathsToPlugins = 1;
    preferences.pathToLogsAndData = logs;
    preferences.featuresToLoad = features;
    preferences.numFeaturesToLoad = 4;
    preferences.engine = sl::EngineType::eCustom;
    preferences.engineVersion = "0.1.0";
    preferences.projectId = "9a49ed89-a846-4ecd-b25a-18461a02f048";
    preferences.renderAPI = sl::RenderAPI::eVulkan;
    if (GetEnvironmentVariableW(L"STREAMLINE_VERBOSE", nullptr, 0)) preferences.logLevel = sl::LogLevel::eVerbose;
    preferences.flags = sl::PreferenceFlags::eDisableCLStateTracking | sl::PreferenceFlags::eUseManualHooking | sl::PreferenceFlags::eUseFrameBasedResourceTagging;
    auto result = p_slInit(preferences, sl::kSDKVersion);
    initialized = result == sl::Result::eOk;
    return static_cast<int>(result);
}

extern "C" void bb_sl_shutdown() {
    if (!initialized) return;
    bb_sl_suspend();
    bb_sl_end();
    p_slShutdown();
    initialized = false;
    // Keep the loader resident: wgpu still uses its Vulkan trampolines during teardown.
}

extern "C" void bb_sl_release() {
    if (initialized) p_slFreeResources(sl::kFeatureDLSS, viewport);
}

extern "C" int bb_sl_supported(void* physical_device) {
    sl::AdapterInfo info{};
    info.vkPhysicalDevice = physical_device;
    auto result = p_slIsFeatureSupported(sl::kFeatureDLSS, info);
    retain_plugin_code();
    return static_cast<int>(result);
}

extern "C" uint64_t bb_sl_evaluations() { return evaluations; }

// Referenced only by GPU tests; no user-visible helper window or message loop is needed.
extern "C" intptr_t bb_sl_test_instance() { return reinterpret_cast<intptr_t>(GetModuleHandleW(nullptr)); }
extern "C" intptr_t bb_sl_test_window_create() {
    WNDCLASSW window_class{};
    window_class.lpfnWndProc = DefWindowProcW;
    window_class.hInstance = GetModuleHandleW(nullptr);
    window_class.lpszClassName = L"BlackBoxStreamlineValidation";
    RegisterClassW(&window_class);
    return reinterpret_cast<intptr_t>(CreateWindowExW(0, window_class.lpszClassName,
        L"BlackBox DLSS validation", WS_POPUP, 0, 0, 640, 360, nullptr, nullptr, window_class.hInstance, nullptr));
}

extern "C" void bb_sl_test_window_resize(intptr_t window, uint32_t width, uint32_t height) {
    SetWindowPos(reinterpret_cast<HWND>(window), nullptr, 0, 0, width, height, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
}

extern "C" void bb_sl_test_window_visible(intptr_t window) {
    auto hwnd = reinterpret_cast<HWND>(window);
    ShowWindow(hwnd, SW_SHOW);
    auto foreground = GetWindowThreadProcessId(GetForegroundWindow(), nullptr);
    auto thread = GetCurrentThreadId();
    bool attached = foreground != thread && AttachThreadInput(thread, foreground, TRUE);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    if (attached) AttachThreadInput(thread, foreground, FALSE);
}

extern "C" void bb_sl_test_window_pump() {
    MSG message{};
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

extern "C" void bb_sl_test_window_destroy(intptr_t window) {
    DestroyWindow(reinterpret_cast<HWND>(window));
}

static sl::DLSSOptions options(uint32_t mode, uint32_t width, uint32_t height) {
    sl::DLSSOptions result{};
    result.mode = static_cast<sl::DLSSMode>(mode);
    result.outputWidth = width;
    result.outputHeight = height;
    result.colorBuffersHDR = sl::Boolean::eTrue;
    result.useAutoExposure = sl::Boolean::eTrue;
    // Explicitly select the DLSS 4.5 second-generation transformer for every supported mode.
    result.dlaaPreset = sl::DLSSPreset::ePresetM;
    result.qualityPreset = sl::DLSSPreset::ePresetM;
    result.balancedPreset = sl::DLSSPreset::ePresetM;
    result.performancePreset = sl::DLSSPreset::ePresetM;
    result.ultraPerformancePreset = sl::DLSSPreset::ePresetL;
    return result;
}

extern "C" int bb_sl_size(uint32_t mode, uint32_t width, uint32_t height, uint32_t* render_size) {
    auto opt = options(mode, width, height);
    sl::DLSSOptimalSettings settings{};
    PFun_slDLSSGetOptimalSettings* function{};
    auto result = p_slGetFeatureFunction(sl::kFeatureDLSS, "slDLSSGetOptimalSettings", reinterpret_cast<void*&>(function));
    if (result == sl::Result::eOk) result = function(opt, settings);
    render_size[0] = settings.optimalRenderWidth;
    render_size[1] = settings.optimalRenderHeight;
    return static_cast<int>(result);
}

struct Image {
    uint64_t image, view;
    uint32_t width, height, format, layout, usage;
};
struct Frame {
    float projection[16], inverse_projection[16], reprojection[16], inverse_reprojection[16], camera_to_world[16];
    float jitter[2], near_plane, fov, delta_ms;
    uint32_t mode, reset;
};

static sl::Result common_constants(const Image* images, const Frame* frame) {
    if (constants_set) return sl::Result::eOk;
    sl::Constants constants{};
    std::memcpy(&constants.cameraViewToClip, frame->projection, sizeof(frame->projection));
    std::memcpy(&constants.clipToCameraView, frame->inverse_projection, sizeof(frame->inverse_projection));
    constants.clipToLensClip = {{{1, 0, 0, 0}, {0, 1, 0, 0}, {0, 0, 1, 0}, {0, 0, 0, 1}}};
    std::memcpy(&constants.clipToPrevClip, frame->reprojection, sizeof(frame->reprojection));
    std::memcpy(&constants.prevClipToClip, frame->inverse_reprojection, sizeof(frame->inverse_reprojection));
    constants.jitterOffset = {frame->jitter[0], frame->jitter[1]};
    constants.cameraPinholeOffset = {0.0f, 0.0f};
    const auto* camera = frame->camera_to_world;
    constants.cameraPos = {camera[12], camera[13], camera[14]};
    constants.cameraRight = {camera[0], camera[1], camera[2]};
    constants.cameraUp = {camera[4], camera[5], camera[6]};
    constants.cameraFwd = {-camera[8], -camera[9], -camera[10]};
    // Native motion is previous-current NDC, with Y pointing up; SL uses UV motion pointing down.
    constants.mvecScale = {0.5f, -0.5f};
    constants.depthInverted = sl::Boolean::eTrue;
    constants.cameraMotionIncluded = sl::Boolean::eTrue;
    constants.motionVectors3D = sl::Boolean::eFalse;
    constants.reset = frame->reset ? sl::Boolean::eTrue : sl::Boolean::eFalse;
    constants.cameraNear = frame->near_plane;
    constants.cameraFar = std::numeric_limits<float>::infinity();
    constants.cameraFOV = frame->fov;
    constants.cameraAspectRatio = float(images[0].width) / float(images[0].height);

    auto result = p_slSetConstants(constants, *current_frame, viewport);
    constants_set = result == sl::Result::eOk;
    return result;
}

extern "C" int bb_sl_evaluate(void* command, const Image* images, const Frame* frame) {
    auto opt = options(frame->mode, images[3].width, images[3].height);
    PFun_slDLSSSetOptions* function{};
    auto result = p_slGetFeatureFunction(sl::kFeatureDLSS, "slDLSSSetOptions", reinterpret_cast<void*&>(function));
    if (result == sl::Result::eOk) result = function(viewport, opt);
    if (result != sl::Result::eOk) return static_cast<int>(result);
    if (!current_frame) bb_sl_begin(1, 0);
    if (!current_frame) return -1;
    result = common_constants(images, frame);
    if (result != sl::Result::eOk) return static_cast<int>(result);
    sl::Resource resources[4];
    sl::BufferType types[] = {sl::kBufferTypeScalingInputColor, sl::kBufferTypeDepth,
        sl::kBufferTypeMotionVectors, sl::kBufferTypeScalingOutputColor};
    sl::ResourceTag tags[] = {
        {&resources[0], types[0], sl::ResourceLifecycle::eValidUntilEvaluate},
        {&resources[1], types[1], sl::ResourceLifecycle::eValidUntilEvaluate},
        {&resources[2], types[2], sl::ResourceLifecycle::eValidUntilEvaluate},
        {&resources[3], types[3], sl::ResourceLifecycle::eValidUntilEvaluate}};
    for (uint32_t i = 0; i < 4; ++i) {
        resources[i].native = reinterpret_cast<void*>(images[i].image);
        resources[i].view = reinterpret_cast<void*>(images[i].view);
        resources[i].width = images[i].width;
        resources[i].height = images[i].height;
        resources[i].nativeFormat = images[i].format;
        resources[i].state = images[i].layout;
        resources[i].usage = images[i].usage;
        resources[i].mipLevels = 1;
        resources[i].arrayLayers = 1;
        tags[i].extent = {0, 0, images[i].width, images[i].height};
    }
    result = p_slSetTagForFrame(*current_frame, viewport, tags, 4, command);
    if (result != sl::Result::eOk) return static_cast<int>(result);
    const sl::BaseStructure* inputs[] = {&viewport};
    result = p_slEvaluateFeature(sl::kFeatureDLSS, *current_frame, inputs, 1, command);
    if (result == sl::Result::eOk) ++evaluations;
    return static_cast<int>(result);
}

// Shared private SDK state; kept in a separate source file for the presentation domain.
#include "presentation.cpp"
