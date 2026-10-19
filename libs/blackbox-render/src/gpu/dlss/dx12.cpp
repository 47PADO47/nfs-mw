// Native D3D12 command/presentation ownership. wgpu owns rendering resources and its queue.
// Streamline 2.14.16 manual-hooking and DLSS-G input-fence contracts (NVIDIA, MIT).
#include <d3d12.h>
#include <dxgi1_6.h>
struct DxSlot {
    ID3D12CommandAllocator* allocator{};
    ID3D12GraphicsCommandList* commands{};
    uint64_t serial{};
};
struct DxContext {
    ID3D12Device* device{};
    ID3D12Device* proxy{};
    ID3D12CommandQueue* render{};
    ID3D12CommandQueue* present{};
    IDXGIFactory4* factory{};
    IDXGISwapChain3* swapchain{};
    ID3D12Fence* fence{};
    HANDLE event{};
    LUID luid{};
    DxSlot evaluation[3]{}, copies[3]{};
    uint64_t serial{};
    uint32_t next_evaluation{}, next_copy{};
    bool tearing{}, ready{};
};
static DxContext dx;
static HRESULT dx_wait(uint64_t value) {
    if (!value) return S_OK;
    const uint64_t completed = dx.fence->GetCompletedValue();
    if (completed == UINT64_MAX) return dx.device->GetDeviceRemovedReason();
    if (completed >= value) return S_OK;
    HRESULT result = dx.fence->SetEventOnCompletion(value, dx.event);
    if (FAILED(result)) return result;
    return WaitForSingleObject(dx.event, 10000) == WAIT_OBJECT_0 ? S_OK : HRESULT_FROM_WIN32(ERROR_TIMEOUT);
}
static HRESULT dx_drain() {
    if (!dx.fence) return S_OK;
    HRESULT result = dx.render->Signal(dx.fence, ++dx.serial);
    if (FAILED(result)) return result;
    result = dx.present->Wait(dx.fence, dx.serial);
    if (FAILED(result)) return result;
    result = dx.present->Signal(dx.fence, ++dx.serial);
    if (FAILED(result)) return result;
    return dx_wait(dx.serial);
}
template<class T> static void dx_release(T*& resource) {
    if (!resource) return;
    resource->Release();
    resource = nullptr;
}
extern "C" void bb_sl_dx12_shutdown() {
    dx_drain();
    dx_release(dx.swapchain);
    for (auto slots : {dx.evaluation, dx.copies}) {
        for (uint32_t i = 0; i < 3; ++i) {
            dx_release(slots[i].commands);
            dx_release(slots[i].allocator);
        }
    }
    dx_release(dx.factory);
    dx_release(dx.fence);
    dx_release(dx.present);
    dx_release(dx.proxy);
    dx_release(dx.render);
    dx_release(dx.device);
    if (dx.event) CloseHandle(dx.event);
    dx.event = nullptr;
}
static HRESULT dx_slot(DxSlot& slot) {
    HRESULT result = dx_wait(slot.serial);
    if (FAILED(result)) return result;
    if (!slot.allocator) {
        result = dx.device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&slot.allocator));
        if (FAILED(result)) return result;
        result = dx.device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, slot.allocator, nullptr, IID_PPV_ARGS(&slot.commands));
        return result;
    }
    result = slot.allocator->Reset();
    if (FAILED(result)) return result;
    return slot.commands->Reset(slot.allocator, nullptr);
}
extern "C" int bb_sl_dx12_attach(void* device, void* queue) {
    if (dx.device) return dx.ready ? 0 : -1;
    auto set_device = reinterpret_cast<PFun_slSetD3DDevice*>(GetProcAddress(module, "slSetD3DDevice"));
    auto upgrade = reinterpret_cast<PFun_slUpgradeInterface*>(GetProcAddress(module, "slUpgradeInterface"));
    if (!set_device || !upgrade) return -1;
    dx.device = static_cast<ID3D12Device*>(device); dx.device->AddRef();
    dx.render = static_cast<ID3D12CommandQueue*>(queue); dx.render->AddRef();
    dx.luid = dx.device->GetAdapterLuid();
    auto result = set_device(device);
    if (result != sl::Result::eOk) return static_cast<int>(result);
    ID3D12Device* proxy = dx.device; proxy->AddRef();
    result = upgrade(reinterpret_cast<void**>(&proxy));
    if (result != sl::Result::eOk) { proxy->Release(); return static_cast<int>(result); }
    D3D12_COMMAND_QUEUE_DESC desc{}; desc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    dx.proxy = proxy;
    HRESULT hr = proxy->CreateCommandQueue(&desc, IID_PPV_ARGS(&dx.present));
    if (FAILED(hr)) return hr;
    hr = dx.device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&dx.fence));
    if (FAILED(hr)) return hr;
    dx.event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!dx.event) return HRESULT_FROM_WIN32(GetLastError());
    auto library = LoadLibraryW(L"dxgi.dll");
    auto create = reinterpret_cast<HRESULT(WINAPI*)(UINT, REFIID, void**)>(GetProcAddress(library, "CreateDXGIFactory2"));
    if (!create) return -1;
    hr = create(0, IID_PPV_ARGS(&dx.factory));
    if (FAILED(hr)) return hr;
    IDXGIFactory5* factory5{};
    if (SUCCEEDED(dx.factory->QueryInterface(IID_PPV_ARGS(&factory5)))) {
        BOOL tearing{};
        if (SUCCEEDED(factory5->CheckFeatureSupport(DXGI_FEATURE_PRESENT_ALLOW_TEARING, &tearing, sizeof(tearing)))) dx.tearing = tearing;
        factory5->Release();
    }
    result = upgrade(reinterpret_cast<void**>(&dx.factory));
    dx.ready = result == sl::Result::eOk;
    return static_cast<int>(result);
}
static void dx_adapter(sl::AdapterInfo& info) {
    info.deviceLUID = reinterpret_cast<uint8_t*>(&dx.luid);
    info.deviceLUIDSizeInBytes = sizeof(dx.luid);
}
extern "C" int bb_sl_dx12_configure(void* window, uint32_t width, uint32_t height, uint32_t format, uint32_t color_space) {
    HRESULT hr = dx_drain();
    if (FAILED(hr)) return hr;
    const UINT flags = dx.tearing ? DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING : 0;
    if (dx.swapchain) {
        hr = dx.swapchain->ResizeBuffers(3, width, height, static_cast<DXGI_FORMAT>(format), flags);
        if (FAILED(hr)) return hr;
        return dx.swapchain->SetColorSpace1(static_cast<DXGI_COLOR_SPACE_TYPE>(color_space));
    }
    DXGI_SWAP_CHAIN_DESC1 desc{};
    desc.Width = width; desc.Height = height; desc.Format = static_cast<DXGI_FORMAT>(format);
    desc.SampleDesc.Count = 1; desc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    desc.BufferCount = 3; desc.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD; desc.Flags = flags;
    IDXGISwapChain1* swapchain{};
    hr = dx.factory->CreateSwapChainForHwnd(dx.present, static_cast<HWND>(window), &desc, nullptr, nullptr, &swapchain);
    if (FAILED(hr)) return hr;
    hr = swapchain->QueryInterface(IID_PPV_ARGS(&dx.swapchain));
    swapchain->Release();
    if (FAILED(hr)) return hr;
    dx.factory->MakeWindowAssociation(static_cast<HWND>(window), DXGI_MWA_NO_ALT_ENTER);
    return dx.swapchain->SetColorSpace1(static_cast<DXGI_COLOR_SPACE_TYPE>(color_space));
}
extern "C" int bb_sl_dx12_evaluate(const Image* images, const Frame* frame) {
    auto& slot = dx.evaluation[dx.next_evaluation++ % 3];
    HRESULT hr = dx_slot(slot);
    if (FAILED(hr)) return hr;
    int result = bb_sl_evaluate(slot.commands, images, frame);
    hr = slot.commands->Close();
    if (FAILED(hr)) return hr;
    ID3D12CommandList* lists[] = {slot.commands};
    dx.render->ExecuteCommandLists(1, lists);
    slot.serial = ++dx.serial;
    hr = dx.render->Signal(dx.fence, slot.serial);
    if (FAILED(hr)) return hr;
    return result;
}
extern "C" int bb_sl_dx12_present(void* source, uint32_t vsync) {
    if (!dx.swapchain) return -1;
    auto& slot = dx.copies[dx.next_copy++ % 3];
    HRESULT hr = dx_slot(slot);
    if (FAILED(hr)) return hr;
    hr = dx.render->Signal(dx.fence, ++dx.serial);
    if (FAILED(hr)) return hr;
    hr = dx.present->Wait(dx.fence, dx.serial);
    if (FAILED(hr)) return hr;
    ID3D12Resource* buffer{};
    hr = dx.swapchain->GetBuffer(dx.swapchain->GetCurrentBackBufferIndex(), IID_PPV_ARGS(&buffer));
    if (FAILED(hr)) return hr;
    D3D12_RESOURCE_BARRIER barrier{};
    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    barrier.Transition.pResource = buffer;
    barrier.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_PRESENT;
    barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_DEST;
    slot.commands->ResourceBarrier(1, &barrier);
    slot.commands->CopyResource(buffer, static_cast<ID3D12Resource*>(source));
    barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
    barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_PRESENT;
    slot.commands->ResourceBarrier(1, &barrier);
    hr = slot.commands->Close();
    buffer->Release();
    if (FAILED(hr)) return hr;
    ID3D12CommandList* lists[] = {slot.commands};
    dx.present->ExecuteCommandLists(1, lists);
    hr = dx.swapchain->Present(vsync, !vsync && dx.tearing ? DXGI_PRESENT_ALLOW_TEARING : 0);
    if (FAILED(hr)) return hr;
    slot.serial = ++dx.serial;
    hr = dx.present->Signal(dx.fence, slot.serial);
    if (FAILED(hr)) return hr;
    // The render target is reused next frame on a different queue. Keep the copy ordered on GPU.
    return dx.render->Wait(dx.fence, slot.serial);
}
static int dx_inputs_consumed(const sl::DLSSGState& state) {
    if (dx.render && state.inputsProcessingCompletionFence && state.lastPresentInputsProcessingCompletionFenceValue) {
        // SDK requires this for resources reused on any non-presenting client queue.
        return dx.render->Wait(static_cast<ID3D12Fence*>(state.inputsProcessingCompletionFence), state.lastPresentInputsProcessingCompletionFenceValue);
    }
    return 0;
}

extern "C" void* bb_sl_dx12_depth(void* device, uint32_t width, uint32_t height) {
    D3D12_HEAP_PROPERTIES heap{}; heap.Type = D3D12_HEAP_TYPE_DEFAULT;
    D3D12_RESOURCE_DESC desc{};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    desc.Width = width; desc.Height = height; desc.DepthOrArraySize = 1; desc.MipLevels = 1;
    desc.Format = DXGI_FORMAT_R32_TYPELESS; desc.SampleDesc.Count = 1;
    desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL;
    D3D12_CLEAR_VALUE clear{}; clear.Format = DXGI_FORMAT_D32_FLOAT;
    ID3D12Resource* resource{};
    HRESULT hr = static_cast<ID3D12Device*>(device)->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE,
        &desc, D3D12_RESOURCE_STATE_COMMON, &clear, IID_PPV_ARGS(&resource));
    if (FAILED(hr)) return nullptr;
    return resource;
}
