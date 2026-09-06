// Local-only LiteRT worker. The parent owns model selection, lifetime and saving.
// Protocol: ready [magic, input bytes, output bytes, compile ms, threads], then
// command 'T' + one fixed RGB tile -> [inference ms] + fixed RGB output; 'Q'/EOF exits.
#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <cstring>
#include <stdexcept>
#include <vector>
#include <thread>
#include <unistd.h>
#include "litert/c/litert_compiled_model.h"
#include "litert/c/litert_environment.h"
#include "litert/c/litert_model.h"
#include "litert/c/litert_options.h"
#include "litert/c/litert_opaque_options.h"
#include "litert/c/litert_tensor_buffer.h"
#include "litert/c/litert_tensor_buffer_requirements.h"

using Clock = std::chrono::steady_clock;
static uint32_t elapsed(Clock::time_point start) {
    return std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now() - start).count();
}
static void word(uint32_t value) {
    const char bytes[] = {char(value), char(value >> 8), char(value >> 16), char(value >> 24)};
    std::cout.write(bytes, 4);
}
static void check(LiteRtStatus status) {
    if (status != kLiteRtStatusOk) throw std::runtime_error("LiteRT status " + std::to_string(status));
}
struct Runtime {
    LiteRtEnvironment env = nullptr;
    LiteRtModel model = nullptr;
    LiteRtOptions options = nullptr;
    LiteRtCompiledModel compiled = nullptr;
    LiteRtTensorBuffer input = nullptr, output = nullptr;
    ~Runtime() {
        if (input) LiteRtDestroyTensorBuffer(input);
        if (output) LiteRtDestroyTensorBuffer(output);
        if (compiled) LiteRtDestroyCompiledModel(compiled);
        if (options) LiteRtDestroyOptions(options);
        if (model) LiteRtDestroyModel(model);
        if (env) LiteRtDestroyEnvironment(env);
    }
};
static void serve(const char* model_path, int threads) {
    const auto started = Clock::now();
    Runtime runtime;
    check(LiteRtCreateEnvironment(0, nullptr, &runtime.env));
    check(LiteRtCreateModelFromFile(runtime.env, model_path, &runtime.model));
    check(LiteRtCreateOptions(&runtime.options));
    check(LiteRtSetOptionsHardwareAccelerators(runtime.options, kLiteRtHwAcceleratorCpu));
    // Pinned LiteRT CPU options use a TOML C-string payload (upstream
    // litert/c/options/litert_cpu_options.cc). No optional GPU/NPU libraries.
    auto payload = strdup(("num_threads = " + std::to_string(threads) + "\n").c_str());
    if (!payload) throw std::bad_alloc();
    LiteRtOpaqueOptions cpu = nullptr;
    auto status = LiteRtCreateOpaqueOptions("xnnpack", payload, std::free, &cpu);
    if (status != kLiteRtStatusOk) { std::free(payload); check(status); }
    status = LiteRtAddOpaqueOptions(runtime.options, cpu);
    if (status != kLiteRtStatusOk) { LiteRtDestroyOpaqueOptions(cpu); check(status); }
    check(LiteRtCreateCompiledModel(runtime.env, runtime.model, runtime.options, &runtime.compiled));
    LiteRtSignature signature;
    check(LiteRtGetModelSignature(runtime.model, 0, &signature));
    LiteRtParamIndex input_count, output_count;
    check(LiteRtGetNumSignatureInputs(signature, &input_count));
    check(LiteRtGetNumSignatureOutputs(signature, &output_count));
    if (input_count != 1 || output_count != 1) throw std::runtime_error("Invalid tensor count");
    for (int index = 0; index < 2; ++index) {
        LiteRtTensor tensor;
        check(index ? LiteRtGetSignatureOutputTensorByIndex(signature, 0, &tensor) : LiteRtGetSignatureInputTensorByIndex(signature, 0, &tensor));
        LiteRtRankedTensorType type;
        check(LiteRtGetRankedTensorType(tensor, &type));
        const int edge = index ? 512 : 128;
        if (type.element_type != kLiteRtElementTypeUInt8 || type.layout.rank != 4 ||
            type.layout.dimensions[0] != 1 || type.layout.dimensions[1] != edge ||
            type.layout.dimensions[2] != edge || type.layout.dimensions[3] != 3)
            throw std::runtime_error("Unsupported upscaler tensor contract");
        LiteRtTensorBufferRequirements requirements;
        check(index ? LiteRtGetCompiledModelOutputBufferRequirements(runtime.compiled, 0, 0, &requirements) : LiteRtGetCompiledModelInputBufferRequirements(runtime.compiled, 0, 0, &requirements));
        size_t size;
        check(LiteRtGetTensorBufferRequirementsBufferSize(requirements, &size));
        if (size < size_t(edge * edge * 3) || size > 4 * 1024 * 1024) throw std::runtime_error("Invalid buffer size");
        check(LiteRtCreateManagedTensorBuffer(runtime.env, kLiteRtTensorBufferTypeHostMemory, &type, size, index ? &runtime.output : &runtime.input));
    }
    std::vector<uint8_t> input(128 * 128 * 3), output(512 * 512 * 3);
    word(0x3154524c); word(input.size()); word(output.size()); word(elapsed(started)); word(threads);
    std::cout.flush();
    char command;
    while (std::cin.get(command)) {
        if (command == 'Q') return;
        if (command != 'T' || !std::cin.read(reinterpret_cast<char*>(input.data()), input.size()))
            throw std::runtime_error("Invalid tile frame");
        const auto tile_started = Clock::now();
        void* memory;
        check(LiteRtLockTensorBuffer(runtime.input, &memory, kLiteRtTensorBufferLockModeWrite));
        std::memcpy(memory, input.data(), input.size());
        check(LiteRtUnlockTensorBuffer(runtime.input));
        check(LiteRtRunCompiledModel(runtime.compiled, 0, 1, &runtime.input, 1, &runtime.output));
        check(LiteRtLockTensorBuffer(runtime.output, &memory, kLiteRtTensorBufferLockModeRead));
        std::memcpy(output.data(), memory, output.size());
        check(LiteRtUnlockTensorBuffer(runtime.output));
        word(elapsed(tile_started));
        std::cout.write(reinterpret_cast<char*>(output.data()), output.size());
        std::cout.flush();
        if (!std::cout) return;
    }
}
int main(int argc, char** argv) {
    if (argc != 3) return 2;
    if (std::strlen(argv[2]) != 1 || argv[2][0] < '1' || argv[2][0] > '4') return 2;
    const int threads = argv[2][0] - '0';
    // Never leave inference orphaned after a parent crash; no app/Keychain state.
    const pid_t parent = getppid();
    std::thread([parent] {
        for (;;) {
            std::this_thread::sleep_for(std::chrono::seconds(1));
            if (getppid() != parent) _exit(3);
        }
    }).detach();
    try { serve(argv[1], threads); }
    catch (const std::exception& error) { std::cerr << "LiteRT worker failed: " << error.what() << '\n'; return 1; }
    return 0;
}
