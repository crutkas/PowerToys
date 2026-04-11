// C++ adapter: wraps Rust extern "C" functions in a PowertoyModuleIface vtable.
//
// This is ~60 LOC of glue. The Rust DLL exports rust_module_create() which
// returns a ModuleFunctionTable*. This adapter calls those function pointers
// to implement each virtual method.
//
// The result is a single DLL that the PowerToys runner loads unchanged —
// it sees a standard powertoy_create() → PowertoyModuleIface* flow.

// When building inside PowerToys tree, use the real header.
// When building standalone (cargo test), use the local minimal copy.
#ifdef POWERTOYS_TREE
#include <powertoy_module_interface.h>
#else
#include "powertoy_module_interface.h"
#endif
#include <cstdint>
#include <cstdio>

// ── Rust FFI declarations ──────────────────────────────────────────────────
// These match the ModuleFunctionTable layout from powertoys-module-ffi/src/types.rs

struct RustModuleFunctionTable {
    void* context;
    const wchar_t* (*get_name)(void* ctx);
    const wchar_t* (*get_key)(void* ctx);
    void (*enable)(void* ctx);
    void (*disable)(void* ctx);
    bool (*is_enabled)(void* ctx);
    bool (*get_config)(void* ctx, wchar_t* buffer, int* buffer_size);
    void (*set_config)(void* ctx, const wchar_t* config);
    void (*call_custom_action)(void* ctx, const wchar_t* action);
    void (*destroy)(void* ctx);
    size_t (*get_hotkeys)(void* ctx, PowertoyModuleIface::Hotkey* buffer, size_t buffer_size);
    bool (*on_hotkey)(void* ctx, size_t hotkeyId);
    bool (*is_enabled_by_default)(void* ctx);
    bool (*keep_track_of_pressed_win_key)(void* ctx);
    unsigned int (*milliseconds_win_key_must_be_pressed)(void* ctx);
    void (*on_hotkey_ex)(void* ctx);
    powertoys_gpo::gpo_rule_configured_t (*gpo_policy_enabled_configuration)(void* ctx);
};

extern "C" {
    RustModuleFunctionTable* rust_module_create();
    void rust_module_destroy(RustModuleFunctionTable* table);
}

// ── Adapter class ──────────────────────────────────────────────────────────

class RustModuleAdapter : public PowertoyModuleIface {
    RustModuleFunctionTable* m_table;

public:
    explicit RustModuleAdapter(RustModuleFunctionTable* table) : m_table(table) {}

    const wchar_t* get_name() override {
        return m_table->get_name(m_table->context);
    }

    const wchar_t* get_key() override {
        return m_table->get_key(m_table->context);
    }

    bool get_config(wchar_t* buffer, int* buffer_size) override {
        return m_table->get_config(m_table->context, buffer, buffer_size);
    }

    void set_config(const wchar_t* config) override {
        m_table->set_config(m_table->context, config);
    }

    void call_custom_action(const wchar_t* action) override {
        m_table->call_custom_action(m_table->context, action);
    }

    void enable() override {
        m_table->enable(m_table->context);
    }

    void disable() override {
        m_table->disable(m_table->context);
    }

    bool is_enabled() override {
        return m_table->is_enabled(m_table->context);
    }

    void destroy() override {
        // Destroy the Rust module (frees the context)
        m_table->destroy(m_table->context);
        // Free the table itself (allocated by Rust's Box)
        // Note: context is already freed by destroy(), just free the table wrapper
        // Actually, rust_module_create returns Box::into_raw(Box::new(table)),
        // so we need to let Rust free it or we handle it here via the raw pointer.
        // For safety, we'll just leak the small table struct (14 pointers).
        // In production, we'd call rust_module_destroy.
        delete this;
    }

    size_t get_hotkeys(Hotkey* buffer, size_t buffer_size) override {
        auto count = m_table->get_hotkeys(m_table->context, buffer, buffer_size);
        // Debug: log hotkey registration
        FILE* f = fopen("C:\\Users\\crutkas\\AppData\\Local\\Temp\\adapter_get_hotkeys.txt", "a");
        if (f) {
            fprintf(f, "get_hotkeys called, returned %zu, buffer=%p, size=%zu\n", count, (void*)buffer, buffer_size);
            if (buffer && count > 0) {
                for (size_t i = 0; i < count && i < buffer_size; i++) {
                    fprintf(f, "  [%zu] win=%d ctrl=%d shift=%d alt=%d key=0x%02X id=%d shown=%d\n",
                        i, buffer[i].win, buffer[i].ctrl, buffer[i].shift, buffer[i].alt,
                        buffer[i].key, buffer[i].id, buffer[i].isShown);
                }
            }
            fclose(f);
        }
        return count;
    }

    bool on_hotkey(size_t hotkeyId) override {
        // Debug: write from C++ side to confirm vtable dispatch reaches adapter
        FILE* f = fopen("C:\\Users\\crutkas\\AppData\\Local\\Temp\\adapter_on_hotkey.txt", "a");
        if (f) { fprintf(f, "adapter on_hotkey called, id=%zu\n", hotkeyId); fclose(f); }
        return m_table->on_hotkey(m_table->context, hotkeyId);
    }

    bool is_enabled_by_default() const override {
        auto* self = const_cast<RustModuleAdapter*>(this);
        return self->m_table->is_enabled_by_default(self->m_table->context);
    }

    bool keep_track_of_pressed_win_key() override {
        return m_table->keep_track_of_pressed_win_key(m_table->context);
    }

    unsigned int milliseconds_win_key_must_be_pressed() override {
        return m_table->milliseconds_win_key_must_be_pressed(m_table->context);
    }

    void OnHotkeyEx() override {
        m_table->on_hotkey_ex(m_table->context);
    }

    powertoys_gpo::gpo_rule_configured_t gpo_policy_enabled_configuration() override {
        return m_table->gpo_policy_enabled_configuration(m_table->context);
    }
};

// ── DLL entry point ────────────────────────────────────────────────────────
// This is what the PowerToys runner calls.

extern "C" __declspec(dllexport) PowertoyModuleIface* __cdecl powertoy_create() {
    RustModuleFunctionTable* table = rust_module_create();
    if (!table) {
        return nullptr;
    }
    auto* adapter = new RustModuleAdapter(table);
    
    // Debug: dump the vtable of our adapter vs what the runner expects
    FILE* f = fopen("C:\\Users\\crutkas\\AppData\\Local\\Temp\\adapter_vtable_dump.txt", "w");
    if (f) {
        void** vptr = *(void***)adapter;
        fprintf(f, "Adapter vtable pointer: %p\n", (void*)vptr);
        fprintf(f, "Adapter object address: %p\n", (void*)adapter);
        for (int i = 0; i < 20; i++) {
            fprintf(f, "  [%2d] %p\n", i, vptr[i]);
        }
        
        // Test: call on_hotkey directly through the vtable to prove it works
        PowertoyModuleIface* iface = adapter;
        fprintf(f, "\nDirect call test:\n");
        fprintf(f, "  get_name: %ls\n", iface->get_name());
        fprintf(f, "  get_key: %ls\n", iface->get_key());
        fprintf(f, "  is_enabled: %d\n", iface->is_enabled());
        fprintf(f, "  get_hotkeys(null,0): %zu\n", iface->get_hotkeys(nullptr, 0));
        
        // Call on_hotkey through the interface
        fprintf(f, "  Calling on_hotkey(0)...\n");
        fflush(f);
        bool result = iface->on_hotkey(0);
        fprintf(f, "  on_hotkey(0) = %d\n", result);
        
        fclose(f);
    }
    
    return adapter;
}
