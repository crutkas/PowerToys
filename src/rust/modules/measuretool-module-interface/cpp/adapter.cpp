// C++ adapter: wraps Rust extern "C" functions in a PowertoyModuleIface vtable.
//
// The Rust DLL exports rust_module_create() which returns a ModuleFunctionTable*.
// This adapter calls those function pointers to implement each virtual method.
//
// The result is a single DLL that the PowerToys runner loads unchanged —
// it sees a standard powertoy_create() → PowertoyModuleIface* flow.

#ifdef POWERTOYS_TREE
#include <powertoy_module_interface.h>
#else
#include "powertoy_module_interface.h"
#endif
#include <cstdint>
#include <cstdio>

// ── Rust FFI declarations ──────────────────────────────────────────────────

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
    bool (*get_hotkey_ex)(void* ctx, void* out);
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
        m_table->destroy(m_table->context);
        delete this;
    }

    size_t get_hotkeys(Hotkey* buffer, size_t buffer_size) override {
        return m_table->get_hotkeys(m_table->context, buffer, buffer_size);
    }

    bool on_hotkey(size_t hotkeyId) override {
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

    powertoys_gpo::gpo_rule_configured_t gpo_policy_enabled_configuration() override {
        return m_table->gpo_policy_enabled_configuration(m_table->context);
    }
};

// ── DLL entry point ────────────────────────────────────────────────────────

extern "C" __declspec(dllexport) PowertoyModuleIface* __cdecl powertoy_create() {
    RustModuleFunctionTable* table = rust_module_create();
    if (!table) {
        return nullptr;
    }
    return new RustModuleAdapter(table);
}
