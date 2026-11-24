/**
 * @returns {Promise<void>}
 */
export function start() {
    wasm.start();
}
export function __wbg_Error_30c8987f7c2ed4e2(arg0, arg1) {
    const ret = Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}
export function __wbg_String_8564e559799eccda(arg0, arg1) {
    const ret = String(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg___wbindgen_bigint_get_as_i64_a2383202b9353e4c(arg0, arg1) {
    const v = getObject(arg1);
    const ret = typeof(v) === 'bigint' ? v : undefined;
    getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
}
export function __wbg___wbindgen_boolean_get_5b446f51afd21013(arg0) {
    const v = getObject(arg0);
    const ret = typeof(v) === 'boolean' ? v : undefined;
    return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
}
export function __wbg___wbindgen_debug_string_4687d8d8c2017d52(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg___wbindgen_in_92f62ee1427d9e49(arg0, arg1) {
    const ret = getObject(arg0) in getObject(arg1);
    return ret;
}
export function __wbg___wbindgen_is_bigint_b123553bed3bb382(arg0) {
    const ret = typeof(getObject(arg0)) === 'bigint';
    return ret;
}
export function __wbg___wbindgen_is_function_1f9d30630b8b1d3d(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
}
export function __wbg___wbindgen_is_object_3c45d4f2dde4e749(arg0) {
    const val = getObject(arg0);
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
}
export function __wbg___wbindgen_is_undefined_8865fb403f8fe9d8(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
}
export function __wbg___wbindgen_jsval_eq_02babf21faa37971(arg0, arg1) {
    const ret = getObject(arg0) === getObject(arg1);
    return ret;
}
export function __wbg___wbindgen_jsval_loose_eq_677f21e468d6b461(arg0, arg1) {
    const ret = getObject(arg0) == getObject(arg1);
    return ret;
}
export function __wbg___wbindgen_number_get_2e0e7dee9f701a71(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'number' ? obj : undefined;
    getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
}
export function __wbg___wbindgen_rethrow_cb2e88c6b2a16733(arg0) {
    throw takeObject(arg0);
}
export function __wbg___wbindgen_string_get_0380ccaa2f57f0d9(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg___wbindgen_throw_41e9ee4f547fc59a(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
}
export function __wbg__wbg_cb_unref_dcc1a90847f04c41(arg0) {
    getObject(arg0)._wbg_cb_unref();
}
export function __wbg_activeElement_1e45e0df6b6ac829(arg0) {
    const ret = getObject(arg0).activeElement;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_activeElement_69b55e3697f14d9b(arg0) {
    const ret = getObject(arg0).activeElement;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_activeTexture_11079673eac5c921(arg0, arg1) {
    getObject(arg0).activeTexture(arg1 >>> 0);
}
export function __wbg_activeTexture_2fad46c01cafe6b1(arg0, arg1) {
    getObject(arg0).activeTexture(arg1 >>> 0);
}
export function __wbg_addEventListener_4d0db17c671ea324() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments); }
export function __wbg_addEventListener_8d3235978b9c69a8() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
}, arguments); }
export function __wbg_alert_e913f93d6718c2f3() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).alert(getStringFromWasm0(arg1, arg2));
}, arguments); }
export function __wbg_altKey_1e5fe17a42050103(arg0) {
    const ret = getObject(arg0).altKey;
    return ret;
}
export function __wbg_altKey_cfb4c4fab9a7d0d0(arg0) {
    const ret = getObject(arg0).altKey;
    return ret;
}
export function __wbg_appendChild_fb8c52e7dd8484ea() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).appendChild(getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_arrayBuffer_0fe6e1300abcf908(arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
}
export function __wbg_at_31a66e62eefd72ad(arg0, arg1) {
    const ret = getObject(arg0).at(arg1);
    return addHeapObject(ret);
}
export function __wbg_attachShader_8920a5beea54aba7(arg0, arg1, arg2) {
    getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
}
export function __wbg_attachShader_9567ae57af6746d5(arg0, arg1, arg2) {
    getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
}
export function __wbg_bindBuffer_0324f62b876c4403(arg0, arg1, arg2) {
    getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
}
export function __wbg_bindBuffer_2195e89b585ce0c3(arg0, arg1, arg2) {
    getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
}
export function __wbg_bindTexture_3cf008f256f3ed99(arg0, arg1, arg2) {
    getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
}
export function __wbg_bindTexture_613a47b8917a0ccd(arg0, arg1, arg2) {
    getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
}
export function __wbg_bindVertexArrayOES_3141815cb8234a6e(arg0, arg1) {
    getObject(arg0).bindVertexArrayOES(getObject(arg1));
}
export function __wbg_bindVertexArray_f58af0a346185ec5(arg0, arg1) {
    getObject(arg0).bindVertexArray(getObject(arg1));
}
export function __wbg_blendEquationSeparate_0a17fa4ea863d445(arg0, arg1, arg2) {
    getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
}
export function __wbg_blendEquationSeparate_fa2d4e5dde5ad8b3(arg0, arg1, arg2) {
    getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
}
export function __wbg_blendFuncSeparate_28abe082febb1a26(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
}
export function __wbg_blendFuncSeparate_2af9d47f7efc0452(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
}
export function __wbg_blockSize_1e07d486d3f2411b(arg0) {
    const ret = getObject(arg0).blockSize;
    return ret;
}
export function __wbg_blur_cb8bd1ead8743f99() { return handleError(function (arg0) {
    getObject(arg0).blur();
}, arguments); }
export function __wbg_body_e549239eaff082e1(arg0) {
    const ret = getObject(arg0).body;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_bottom_29cd36a415d12885(arg0) {
    const ret = getObject(arg0).bottom;
    return ret;
}
export function __wbg_bufferData_53e0f7a06b58fe81(arg0, arg1, arg2, arg3) {
    getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
}
export function __wbg_bufferData_5fa44d94fe7a3878(arg0, arg1, arg2, arg3) {
    getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
}
export function __wbg_buffer_56ec2905a66f58b9(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
}
export function __wbg_button_1aa380c7ca420cd1(arg0) {
    const ret = getObject(arg0).button;
    return ret;
}
export function __wbg_call_6137034ef55c9d0f() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_cancelAnimationFrame_6d30ac112c49342a() { return handleError(function (arg0, arg1) {
    getObject(arg0).cancelAnimationFrame(arg1);
}, arguments); }
export function __wbg_changedTouches_df984754af5ef0fa(arg0) {
    const ret = getObject(arg0).changedTouches;
    return addHeapObject(ret);
}
export function __wbg_clearColor_3f6aa096dc12e8b7(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
}
export function __wbg_clearColor_796427e8ac60fe14(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
}
export function __wbg_clearInterval_f1a050672da2658c(arg0, arg1) {
    getObject(arg0).clearInterval(arg1);
}
export function __wbg_clearTimeout_c7bc4c3c7a7774af(arg0, arg1) {
    getObject(arg0).clearTimeout(arg1);
}
export function __wbg_clear_175cf3cb6d98d72b(arg0, arg1) {
    getObject(arg0).clear(arg1 >>> 0);
}
export function __wbg_clear_cb96796cc568b7c8(arg0, arg1) {
    getObject(arg0).clear(arg1 >>> 0);
}
export function __wbg_click_d990e70c576b7c54(arg0) {
    getObject(arg0).click();
}
export function __wbg_clientX_5075231ed45e8ef5(arg0) {
    const ret = getObject(arg0).clientX;
    return ret;
}
export function __wbg_clientX_80fda6c64299fd4d(arg0) {
    const ret = getObject(arg0).clientX;
    return ret;
}
export function __wbg_clientY_0206292d4e8f94cb(arg0) {
    const ret = getObject(arg0).clientY;
    return ret;
}
export function __wbg_clientY_846c39814ce02ab2(arg0) {
    const ret = getObject(arg0).clientY;
    return ret;
}
export function __wbg_clipboardData_0048090643ed57a2(arg0) {
    const ret = getObject(arg0).clipboardData;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_clipboard_9bf6a11ffd0b00a8(arg0) {
    const ret = getObject(arg0).clipboard;
    return addHeapObject(ret);
}
export function __wbg_code_422d539f5d7bacbb(arg0, arg1) {
    const ret = getObject(arg1).code;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_colorMask_4d6afbbbc3022fbe(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
}
export function __wbg_colorMask_8c903995f6f873be(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
}
export function __wbg_compileShader_073a75409aaf96d2(arg0, arg1) {
    getObject(arg0).compileShader(getObject(arg1));
}
export function __wbg_compileShader_c2b53ff3a8851f60(arg0, arg1) {
    getObject(arg0).compileShader(getObject(arg1));
}
export function __wbg_contentBoxSize_1c91213ed1ddb06d(arg0) {
    const ret = getObject(arg0).contentBoxSize;
    return addHeapObject(ret);
}
export function __wbg_contentRect_65cd41b1e711b4e1(arg0) {
    const ret = getObject(arg0).contentRect;
    return addHeapObject(ret);
}
export function __wbg_createBuffer_d1dd23bf8704564c(arg0) {
    const ret = getObject(arg0).createBuffer();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createBuffer_ffe613371d42cfd3(arg0) {
    const ret = getObject(arg0).createBuffer();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createElement_74049073a11f9c31() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_createObjectURL_2f3b38279fe61801() { return handleError(function (arg0, arg1) {
    const ret = URL.createObjectURL(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_createProgram_cbf448e6f18a9402(arg0) {
    const ret = getObject(arg0).createProgram();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createProgram_e0c81b0d2b8d0fd0(arg0) {
    const ret = getObject(arg0).createProgram();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createShader_6c1a2295587878a5(arg0, arg1) {
    const ret = getObject(arg0).createShader(arg1 >>> 0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createShader_7c9d99a21ecb08c0(arg0, arg1) {
    const ret = getObject(arg0).createShader(arg1 >>> 0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createTexture_01e8e591b64237de(arg0) {
    const ret = getObject(arg0).createTexture();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createTexture_5f2583508ebe379b(arg0) {
    const ret = getObject(arg0).createTexture();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createVertexArrayOES_a8dc8498b56df937(arg0) {
    const ret = getObject(arg0).createVertexArrayOES();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_createVertexArray_b223ea69dd08fcab(arg0) {
    const ret = getObject(arg0).createVertexArray();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_ctrlKey_282349f48e6cb157(arg0) {
    const ret = getObject(arg0).ctrlKey;
    return ret;
}
export function __wbg_ctrlKey_8b5101745e8782fa(arg0) {
    const ret = getObject(arg0).ctrlKey;
    return ret;
}
export function __wbg_dataTransfer_afa27477f2f9e0e6(arg0) {
    const ret = getObject(arg0).dataTransfer;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_data_522f7abc70721269(arg0) {
    const ret = getObject(arg0).data;
    return addHeapObject(ret);
}
export function __wbg_deleteBuffer_070e58aaaf2a7a8b(arg0, arg1) {
    getObject(arg0).deleteBuffer(getObject(arg1));
}
export function __wbg_deleteBuffer_7775ee9e248c8468(arg0, arg1) {
    getObject(arg0).deleteBuffer(getObject(arg1));
}
export function __wbg_deleteProgram_822906f0368175d0(arg0, arg1) {
    getObject(arg0).deleteProgram(getObject(arg1));
}
export function __wbg_deleteProgram_b524b30ad176cdb4(arg0, arg1) {
    getObject(arg0).deleteProgram(getObject(arg1));
}
export function __wbg_deleteShader_a70f6cc1918d67eb(arg0, arg1) {
    getObject(arg0).deleteShader(getObject(arg1));
}
export function __wbg_deleteShader_da18a294158c2d15(arg0, arg1) {
    getObject(arg0).deleteShader(getObject(arg1));
}
export function __wbg_deleteTexture_78c170241204a466(arg0, arg1) {
    getObject(arg0).deleteTexture(getObject(arg1));
}
export function __wbg_deleteTexture_9f42750adb092e4f(arg0, arg1) {
    getObject(arg0).deleteTexture(getObject(arg1));
}
export function __wbg_deltaMode_6280d8266d5520f5(arg0) {
    const ret = getObject(arg0).deltaMode;
    return ret;
}
export function __wbg_deltaX_d330516e89f03877(arg0) {
    const ret = getObject(arg0).deltaX;
    return ret;
}
export function __wbg_deltaY_7abc8fc9878d0002(arg0) {
    const ret = getObject(arg0).deltaY;
    return ret;
}
export function __wbg_detachShader_1ed15fc2d289cf15(arg0, arg1, arg2) {
    getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
}
export function __wbg_detachShader_67dabaaad80a795e(arg0, arg1, arg2) {
    getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
}
export function __wbg_devicePixelContentBoxSize_83d4c55aa7829e7a(arg0) {
    const ret = getObject(arg0).devicePixelContentBoxSize;
    return addHeapObject(ret);
}
export function __wbg_devicePixelRatio_7d39e9af5448d3d4(arg0) {
    const ret = getObject(arg0).devicePixelRatio;
    return ret;
}
export function __wbg_disableVertexAttribArray_11c9f11bee30c9d8(arg0, arg1) {
    getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
}
export function __wbg_disableVertexAttribArray_c4c8af1f9e4120d4(arg0, arg1) {
    getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
}
export function __wbg_disable_19fe787eccdf0272(arg0, arg1) {
    getObject(arg0).disable(arg1 >>> 0);
}
export function __wbg_disable_81b2cb5f8cea15ae(arg0, arg1) {
    getObject(arg0).disable(arg1 >>> 0);
}
export function __wbg_disconnect_1eac4c705724307d(arg0) {
    getObject(arg0).disconnect();
}
export function __wbg_document_9854e03c05fc8834(arg0) {
    const ret = getObject(arg0).document;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_done_b41a1d26cdb37fb6(arg0) {
    const ret = getObject(arg0).done;
    return ret;
}
export function __wbg_drawElements_c8988ba71de1101a(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
}
export function __wbg_drawElements_cc1e6c631e89eddf(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
}
export function __wbg_elementFromPoint_8f6f88cd3ac4b9e8(arg0, arg1, arg2) {
    const ret = getObject(arg0).elementFromPoint(arg1, arg2);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_elementFromPoint_fb48b4624ec3dd6a(arg0, arg1, arg2) {
    const ret = getObject(arg0).elementFromPoint(arg1, arg2);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_enableVertexAttribArray_1496b78a8dbab9fd(arg0, arg1) {
    getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
}
export function __wbg_enableVertexAttribArray_d52693866bc8b8dd(arg0, arg1) {
    getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
}
export function __wbg_enable_6eeb9c0e9b483cee(arg0, arg1) {
    getObject(arg0).enable(arg1 >>> 0);
}
export function __wbg_enable_cb4fc19b51148c37(arg0, arg1) {
    getObject(arg0).enable(arg1 >>> 0);
}
export function __wbg_entries_fb6397112b1de25f(arg0) {
    const ret = Object.entries(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_error_5a019734875b3676(arg0, arg1) {
    let deferred0_0;
    let deferred0_1;
    try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_export4(deferred0_0, deferred0_1, 1);
    }
}
export function __wbg_files_cb1ce1b82f16c484(arg0) {
    const ret = getObject(arg0).files;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_files_fe03006f0ba1a5f0(arg0) {
    const ret = getObject(arg0).files;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_focus_e1638be0df4aeb8f() { return handleError(function (arg0, arg1) {
    getObject(arg0).focus(getObject(arg1));
}, arguments); }
export function __wbg_force_7632bf44933b4388(arg0) {
    const ret = getObject(arg0).force;
    return ret;
}
export function __wbg_generateMipmap_c63604006bfa52d5(arg0, arg1) {
    getObject(arg0).generateMipmap(arg1 >>> 0);
}
export function __wbg_generateMipmap_cd49f6eec9558721(arg0, arg1) {
    getObject(arg0).generateMipmap(arg1 >>> 0);
}
export function __wbg_getAttribLocation_5071f5fc51f90b1a(arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
    return ret;
}
export function __wbg_getAttribLocation_ee9dfb2293fb9f7f(arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
    return ret;
}
export function __wbg_getBoundingClientRect_57152b1a20f3de34(arg0) {
    const ret = getObject(arg0).getBoundingClientRect();
    return addHeapObject(ret);
}
export function __wbg_getComputedStyle_dd0f456804795fd6() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).getComputedStyle(getObject(arg1));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_getContext_635e36719cad2623() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_getData_bc58070c107b7d8a() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg1).getData(getStringFromWasm0(arg2, arg3));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_getElementById_cc94972b404e4eaa(arg0, arg1, arg2) {
    const ret = getObject(arg0).getElementById(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_getError_d626fee76ccfa3d2(arg0) {
    const ret = getObject(arg0).getError();
    return ret;
}
export function __wbg_getError_dd9903fdd93a6802(arg0) {
    const ret = getObject(arg0).getError();
    return ret;
}
export function __wbg_getExtension_1d23846cc4d1b9ed() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_getExtension_abfda2c5db286b07() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_getItem_4c80b7663197c7ff() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg1).getItem(getStringFromWasm0(arg2, arg3));
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_getParameter_974595a6624c6c10() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).getParameter(arg1 >>> 0);
    return addHeapObject(ret);
}, arguments); }
export function __wbg_getParameter_a97f491753b05668() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).getParameter(arg1 >>> 0);
    return addHeapObject(ret);
}, arguments); }
export function __wbg_getProgramInfoLog_208fb1ff31006495(arg0, arg1, arg2) {
    const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_getProgramInfoLog_3276b294d6d42fe2(arg0, arg1, arg2) {
    const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_getProgramParameter_50bafab01cf88652(arg0, arg1, arg2) {
    const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
    return addHeapObject(ret);
}
export function __wbg_getProgramParameter_fc7124bb9fb7c675(arg0, arg1, arg2) {
    const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
    return addHeapObject(ret);
}
export function __wbg_getPropertyValue_e90a3628c2de4fb6() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg1).getPropertyValue(getStringFromWasm0(arg2, arg3));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_getRootNode_76c323f0bf80bdcd(arg0) {
    const ret = getObject(arg0).getRootNode();
    return addHeapObject(ret);
}
export function __wbg_getShaderInfoLog_7e0cbba353ca95da(arg0, arg1, arg2) {
    const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_getShaderInfoLog_bb64c5a52df0b7a3(arg0, arg1, arg2) {
    const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_getShaderParameter_745acf10b8e45e31(arg0, arg1, arg2) {
    const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
    return addHeapObject(ret);
}
export function __wbg_getShaderParameter_e55ea9c456df5484(arg0, arg1, arg2) {
    const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
    return addHeapObject(ret);
}
export function __wbg_getSupportedExtensions_1459bad37860c0c3(arg0) {
    const ret = getObject(arg0).getSupportedExtensions();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_getSupportedExtensions_5ff7ed28c30a8baa(arg0) {
    const ret = getObject(arg0).getSupportedExtensions();
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_getUniformLocation_50ba45dade572703(arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_getUniformLocation_cb4e3e121235f91a(arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_get_31af05bd4842a84f() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_get_5dde8a29f073d908(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_get_658f6698067d9515() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_get_6c896e0571ddae51(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
}
export function __wbg_get_9356b22e2a46465d(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_get_b64ce72bb2abf052(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_get_unchecked_288889d017702237(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
}
export function __wbg_hash_c4083658cccb641b() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).hash;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_height_c9feb8f4a9a61abc(arg0) {
    const ret = getObject(arg0).height;
    return ret;
}
export function __wbg_height_fc97e1a0c2e7331f(arg0) {
    const ret = getObject(arg0).height;
    return ret;
}
export function __wbg_hidden_2ba371ecafe013fa(arg0) {
    const ret = getObject(arg0).hidden;
    return ret;
}
export function __wbg_host_75aa3342c78db1a2() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).host;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_hostname_b9a71dd3c2da0eef() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).hostname;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_href_abc902beb29d526e() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).href;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_id_957842593c7d0e05(arg0, arg1) {
    const ret = getObject(arg1).id;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_identifier_24ee7ee6118f12a7(arg0) {
    const ret = getObject(arg0).identifier;
    return ret;
}
export function __wbg_inlineSize_edbf0c52598aa763(arg0) {
    const ret = getObject(arg0).inlineSize;
    return ret;
}
export function __wbg_inputType_bcd41e55691a33d9(arg0, arg1) {
    const ret = getObject(arg1).inputType;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_insertBefore_f1ba67809033f6b1() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).insertBefore(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_instanceof_ArrayBuffer_a99f175873e5d9b8(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ArrayBuffer;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Document_b2a06e9deaeb61d1(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Document;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Element_818e11074cdb63b5(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Element;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlAnchorElement_e2e601da36602959(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLAnchorElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlCanvasElement_0a30c11fbbf41841(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLCanvasElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlInputElement_5c33d1de59c09c49(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLInputElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Map_b2611749102d7ba3(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Map;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_ResizeObserverEntry_159abe6614a8e952(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ResizeObserverEntry;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_ResizeObserverSize_f9d47e3f8cfd50db(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ResizeObserverSize;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_ShadowRoot_0ef114d846d9fd90(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ShadowRoot;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Uint8Array_828cef2aaacafc31(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Uint8Array;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_WebGl2RenderingContext_64c994c14c0790fb(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof WebGL2RenderingContext;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_WebGlRenderingContext_5e5e16df2e7a4904(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof WebGLRenderingContext;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Window_82d71df4eddf88bc(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Window;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_isArray_e15a2ff68ffdbef2(arg0) {
    const ret = Array.isArray(getObject(arg0));
    return ret;
}
export function __wbg_isComposing_600c9236938f8e5a(arg0) {
    const ret = getObject(arg0).isComposing;
    return ret;
}
export function __wbg_isComposing_d03a4fd2d6155b14(arg0) {
    const ret = getObject(arg0).isComposing;
    return ret;
}
export function __wbg_isSafeInteger_717808ad6a54bd9e(arg0) {
    const ret = Number.isSafeInteger(getObject(arg0));
    return ret;
}
export function __wbg_isSecureContext_e61363f740acf8de(arg0) {
    const ret = getObject(arg0).isSecureContext;
    return ret;
}
export function __wbg_is_4b278c0bd3caba97(arg0, arg1) {
    const ret = Object.is(getObject(arg0), getObject(arg1));
    return ret;
}
export function __wbg_item_0c8f652c0830e437(arg0, arg1) {
    const ret = getObject(arg0).item(arg1 >>> 0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_items_25f8ccc11891dbeb(arg0) {
    const ret = getObject(arg0).items;
    return addHeapObject(ret);
}
export function __wbg_iterator_e3c31c892080e444() {
    const ret = Symbol.iterator;
    return addHeapObject(ret);
}
export function __wbg_keyCode_fbee6c8fd374ff5e(arg0) {
    const ret = getObject(arg0).keyCode;
    return ret;
}
export function __wbg_key_1193871533b99ae5(arg0, arg1) {
    const ret = getObject(arg1).key;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_left_e0a244490fe2f293(arg0) {
    const ret = getObject(arg0).left;
    return ret;
}
export function __wbg_length_06ca2697fe194eac(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_length_357ad61f15f550be(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_length_7f3c00c40364105e(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_length_d4bdea10311bd9cf(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_length_ff6b2e0dd257ad9a(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_linkProgram_571a9ea00d38ff83(arg0, arg1) {
    getObject(arg0).linkProgram(getObject(arg1));
}
export function __wbg_linkProgram_acdc299c43d734a0(arg0, arg1) {
    getObject(arg0).linkProgram(getObject(arg1));
}
export function __wbg_localStorage_567c24950edbc178() { return handleError(function (arg0) {
    const ret = getObject(arg0).localStorage;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_location_9c46e8ada95d3174(arg0) {
    const ret = getObject(arg0).location;
    return addHeapObject(ret);
}
export function __wbg_matchMedia_8a4857f947f11f82() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_matches_778d0873b064b1bf(arg0) {
    const ret = getObject(arg0).matches;
    return ret;
}
export function __wbg_metaKey_9c3be2ab835b0148(arg0) {
    const ret = getObject(arg0).metaKey;
    return ret;
}
export function __wbg_metaKey_fb78da4d237d5bbb(arg0) {
    const ret = getObject(arg0).metaKey;
    return ret;
}
export function __wbg_name_06fefa2542c6ce51(arg0, arg1) {
    const ret = getObject(arg1).name;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_navigator_2156486643462a87(arg0) {
    const ret = getObject(arg0).navigator;
    return addHeapObject(ret);
}
export function __wbg_new_1a8d2827bd9e1a2e() { return handleError(function (arg0) {
    const ret = new ResizeObserver(getObject(arg0));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_new_1dbf7428bba60a42(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_new_617a8cdb8bb1130e() {
    const ret = new Object();
    return addHeapObject(ret);
}
export function __wbg_new_9090cad7737fef55() {
    const ret = new Error();
    return addHeapObject(ret);
}
export function __wbg_new_ee2291f50781bf1d() {
    const ret = new Array();
    return addHeapObject(ret);
}
export function __wbg_new_from_slice_9a868026ffa4208a(arg0, arg1) {
    const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
    return addHeapObject(ret);
}
export function __wbg_new_with_record_from_str_to_blob_promise_d8b08e210be436ad() { return handleError(function (arg0) {
    const ret = new ClipboardItem(getObject(arg0));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_new_with_u8_array_sequence_and_options_fccc08fe93f4b8fd() { return handleError(function (arg0, arg1) {
    const ret = new Blob(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_new_with_u8_array_sequence_f380e3e783ae5617() { return handleError(function (arg0) {
    const ret = new Blob(getObject(arg0));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_nextSibling_e5ecc9bf11c2365d(arg0) {
    const ret = getObject(arg0).nextSibling;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_next_33784799010f1bbe(arg0) {
    const ret = getObject(arg0).next;
    return addHeapObject(ret);
}
export function __wbg_next_f4aac29c42af995c() { return handleError(function (arg0) {
    const ret = getObject(arg0).next();
    return addHeapObject(ret);
}, arguments); }
export function __wbg_now_d0b7f4bea9f38490(arg0) {
    const ret = getObject(arg0).now();
    return ret;
}
export function __wbg_now_e7c6795a7f81e10f(arg0) {
    const ret = getObject(arg0).now();
    return ret;
}
export function __wbg_observe_d2d34ca6fcb958b1(arg0, arg1, arg2) {
    getObject(arg0).observe(getObject(arg1), getObject(arg2));
}
export function __wbg_of_d5dbecdc59e02e34(arg0) {
    const ret = Array.of(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_offsetLeft_eef008f2ccb3b242(arg0) {
    const ret = getObject(arg0).offsetLeft;
    return ret;
}
export function __wbg_offsetTop_c36b688834dcdce3(arg0) {
    const ret = getObject(arg0).offsetTop;
    return ret;
}
export function __wbg_open_7456ad5023904d56() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_open_d426932d10a5f130() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_origin_96d72883773c6d82() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).origin;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_parentNode_32afd0d7ad7dca98(arg0) {
    const ret = getObject(arg0).parentNode;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_parent_5e5c8c3dd5ae0732() { return handleError(function (arg0) {
    const ret = getObject(arg0).parent;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_performance_3fcf6e32a7e1ed0a(arg0) {
    const ret = getObject(arg0).performance;
    return addHeapObject(ret);
}
export function __wbg_performance_d0d7b03fc649b694(arg0) {
    const ret = getObject(arg0).performance;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_pixelStorei_af18e636b3e7b725(arg0, arg1, arg2) {
    getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
}
export function __wbg_pixelStorei_cb4e43466f5b8703(arg0, arg1, arg2) {
    getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
}
export function __wbg_port_73ba89eedf847423() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).port;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_postMessage_92c24d418aca6785() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).postMessage(getObject(arg1), getStringFromWasm0(arg2, arg3));
}, arguments); }
export function __wbg_preventDefault_af59afb0f0a02e20(arg0) {
    getObject(arg0).preventDefault();
}
export function __wbg_protocol_c29b929bd710072b() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).protocol;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_prototypesetcall_bc27214492979395(arg0, arg1, arg2) {
    Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
}
export function __wbg_push_2baf45db356cf468(arg0, arg1) {
    const ret = getObject(arg0).push(getObject(arg1));
    return ret;
}
export function __wbg_queueMicrotask_9833f9a49df95a49(arg0) {
    const ret = getObject(arg0).queueMicrotask;
    return addHeapObject(ret);
}
export function __wbg_queueMicrotask_a72f977e97f23c5f(arg0) {
    queueMicrotask(getObject(arg0));
}
export function __wbg_readPixels_31e44e70558ac590() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
    getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
}, arguments); }
export function __wbg_readPixels_3321086a7818a739() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
    getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
}, arguments); }
export function __wbg_readPixels_eb030cd9976eeca5() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
    getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
}, arguments); }
export function __wbg_removeChild_3745fc2545da50fa() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).removeChild(getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_removeEventListener_6e68185345978771() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments); }
export function __wbg_removeItem_f7b602722ed566dd() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).removeItem(getStringFromWasm0(arg1, arg2));
}, arguments); }
export function __wbg_remove_9a5288388316028c(arg0) {
    getObject(arg0).remove();
}
export function __wbg_requestAnimationFrame_7b526ab7aa550c74() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
    return ret;
}, arguments); }
export function __wbg_resolve_0076e10020304ede(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_revokeObjectURL_479ff72c2d109d80() { return handleError(function (arg0, arg1) {
    URL.revokeObjectURL(getStringFromWasm0(arg0, arg1));
}, arguments); }
export function __wbg_right_1dca6be6342de981(arg0) {
    const ret = getObject(arg0).right;
    return ret;
}
export function __wbg_scissor_12ddc324094acd50(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).scissor(arg1, arg2, arg3, arg4);
}
export function __wbg_scissor_a9468482697c2bdb(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).scissor(arg1, arg2, arg3, arg4);
}
export function __wbg_search_31c06ba77ebf51f4() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).search;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_selectionEnd_582c88abcc45a9f3() { return handleError(function (arg0) {
    const ret = getObject(arg0).selectionEnd;
    return isLikeNone(ret) ? Number.MAX_SAFE_INTEGER : (ret) >>> 0;
}, arguments); }
export function __wbg_selectionStart_b1db69e8858218c6() { return handleError(function (arg0) {
    const ret = getObject(arg0).selectionStart;
    return isLikeNone(ret) ? Number.MAX_SAFE_INTEGER : (ret) >>> 0;
}, arguments); }
export function __wbg_setAttribute_9e7d603908f63705() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_setItem_c5fb0966b484ecd3() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_setProperty_097bc3d55ce44513() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_setTimeout_db7bbc18a17e152a() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
    return ret;
}, arguments); }
export function __wbg_set_145a351398b48c65() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
    return ret;
}, arguments); }
export function __wbg_set_accept_06d626c479e21e36(arg0, arg1, arg2) {
    getObject(arg0).accept = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_box_921bf38648f10dfe(arg0, arg1) {
    getObject(arg0).box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
}
export function __wbg_set_download_327e0fbad75893f0(arg0, arg1, arg2) {
    getObject(arg0).download = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_height_c9789c1c77eaedff(arg0, arg1) {
    getObject(arg0).height = arg1 >>> 0;
}
export function __wbg_set_href_f7219c6cbf6bedc3(arg0, arg1, arg2) {
    getObject(arg0).href = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_id_60955e6018d03b26(arg0, arg1, arg2) {
    getObject(arg0).id = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_once_8c45bbbfb71d1b34(arg0, arg1) {
    getObject(arg0).once = arg1 !== 0;
}
export function __wbg_set_prevent_scroll_aaa386836725b8b4(arg0, arg1) {
    getObject(arg0).preventScroll = arg1 !== 0;
}
export function __wbg_set_tabIndex_6999832ea5d5197e(arg0, arg1) {
    getObject(arg0).tabIndex = arg1;
}
export function __wbg_set_type_19b6e3a56c3fa1f1(arg0, arg1, arg2) {
    getObject(arg0).type = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_type_dcf75b2afa60c694(arg0, arg1, arg2) {
    getObject(arg0).type = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_value_fbd659f94bfb9181(arg0, arg1, arg2) {
    getObject(arg0).value = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_width_b0e1267db4b196b5(arg0, arg1) {
    getObject(arg0).width = arg1 >>> 0;
}
export function __wbg_shaderSource_06379dac249994dd(arg0, arg1, arg2, arg3) {
    getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
}
export function __wbg_shaderSource_c8812639b47d87b9(arg0, arg1, arg2, arg3) {
    getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
}
export function __wbg_shiftKey_be6d5295ee392b33(arg0) {
    const ret = getObject(arg0).shiftKey;
    return ret;
}
export function __wbg_shiftKey_ecdc496db72e8a50(arg0) {
    const ret = getObject(arg0).shiftKey;
    return ret;
}
export function __wbg_size_8f1c0b1a1fbb3810(arg0) {
    const ret = getObject(arg0).size;
    return ret;
}
export function __wbg_stack_0e9ceb67ce7245b8(arg0, arg1) {
    const ret = getObject(arg1).stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_static_accessor_GLOBAL_266715b9d96ba635() {
    const ret = typeof global === 'undefined' ? null : global;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_GLOBAL_THIS_10fb7dc1ae063179() {
    const ret = typeof globalThis === 'undefined' ? null : globalThis;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_SELF_0b583911f537483a() {
    const ret = typeof self === 'undefined' ? null : self;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_WINDOW_d7f903d1508cbdc4() {
    const ret = typeof window === 'undefined' ? null : window;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_stopPropagation_effafbc3bc773f1c(arg0) {
    getObject(arg0).stopPropagation();
}
export function __wbg_style_4bce24230e493a7c(arg0) {
    const ret = getObject(arg0).style;
    return addHeapObject(ret);
}
export function __wbg_texImage2D_82bf2b4c8aa9a1db() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
}, arguments); }
export function __wbg_texImage2D_a856af552acdc9d9() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
}, arguments); }
export function __wbg_texImage2D_ca3d0fe24620ec85() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
}, arguments); }
export function __wbg_texParameteri_647fdb1426809cb5(arg0, arg1, arg2, arg3) {
    getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
}
export function __wbg_texParameteri_7864076abcdb03e0(arg0, arg1, arg2, arg3) {
    getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
}
export function __wbg_texSubImage2D_31a0c2797aa04f32() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
}, arguments); }
export function __wbg_texSubImage2D_5e90c7a719d40256() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
}, arguments); }
export function __wbg_texSubImage2D_bb322aae90f7657f() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
    getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
}, arguments); }
export function __wbg_then_c949d5a25a4e78f8(arg0, arg1, arg2) {
    const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}
export function __wbg_then_e71170d78fcf8954(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
}
export function __wbg_top_ff4627294d2cdeb8(arg0) {
    const ret = getObject(arg0).top;
    return ret;
}
export function __wbg_touches_7b36d35cdb1673fa(arg0) {
    const ret = getObject(arg0).touches;
    return addHeapObject(ret);
}
export function __wbg_type_d44713a24aa37493(arg0, arg1) {
    const ret = getObject(arg1).type;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_uniform1i_531b98c4a7177366(arg0, arg1, arg2) {
    getObject(arg0).uniform1i(getObject(arg1), arg2);
}
export function __wbg_uniform1i_fc2dc8cadc49d8ec(arg0, arg1, arg2) {
    getObject(arg0).uniform1i(getObject(arg1), arg2);
}
export function __wbg_uniform2f_131da2d4bf17e4b9(arg0, arg1, arg2, arg3) {
    getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
}
export function __wbg_uniform2f_a92b5bb95dc5b4d9(arg0, arg1, arg2, arg3) {
    getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
}
export function __wbg_useProgram_5bdae58fd9f9826f(arg0, arg1) {
    getObject(arg0).useProgram(getObject(arg1));
}
export function __wbg_useProgram_e075c2902e420761(arg0, arg1) {
    getObject(arg0).useProgram(getObject(arg1));
}
export function __wbg_userAgent_cde4c65cc98db425() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).userAgent;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_value_05305a761dfa3e0e(arg0, arg1) {
    const ret = getObject(arg1).value;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}
export function __wbg_value_f3c585ee8f5ba40c(arg0) {
    const ret = getObject(arg0).value;
    return addHeapObject(ret);
}
export function __wbg_vertexAttribPointer_24b495e18df325dc(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
}
export function __wbg_vertexAttribPointer_54d56b18f71a946d(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
}
export function __wbg_viewport_16936692526e2ce6(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).viewport(arg1, arg2, arg3, arg4);
}
export function __wbg_viewport_e35d16c542d2ce70(arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).viewport(arg1, arg2, arg3, arg4);
}
export function __wbg_warn_13abc63e0d4b3527(arg0) {
    console.warn(getObject(arg0));
}
export function __wbg_width_3d0dce3d9892e35e(arg0) {
    const ret = getObject(arg0).width;
    return ret;
}
export function __wbg_width_bf3733dbc880874d(arg0) {
    const ret = getObject(arg0).width;
    return ret;
}
export function __wbg_writeText_79a2b37e525e9424(arg0, arg1, arg2) {
    const ret = getObject(arg0).writeText(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}
export function __wbg_write_f88c29d142d623bd(arg0, arg1) {
    const ret = getObject(arg0).write(getObject(arg1));
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000001(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 2617, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_11931);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000002(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Array<any>")], shim_idx: 573, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_2654);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000003(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 573, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_2654_3);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000004(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 2047, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
    const ret = makeClosure(arg0, arg1, __wasm_bindgen_func_elem_8937);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000005(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 2049, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_8936);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000006(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 571, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_2653);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000007(arg0) {
    // Cast intrinsic for `I64 -> Externref`.
    const ret = arg0;
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000008(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
    const ret = getArrayF32FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000009(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(I16)) -> NamedExternref("Int16Array")`.
    const ret = getArrayI16FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000a(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(I32)) -> NamedExternref("Int32Array")`.
    const ret = getArrayI32FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000b(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(I8)) -> NamedExternref("Int8Array")`.
    const ret = getArrayI8FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000c(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(U16)) -> NamedExternref("Uint16Array")`.
    const ret = getArrayU16FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000d(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
    const ret = getArrayU32FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000e(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
    const ret = getArrayU8FromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_000000000000000f(arg0, arg1) {
    // Cast intrinsic for `Ref(String) -> Externref`.
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000010(arg0) {
    // Cast intrinsic for `U64 -> Externref`.
    const ret = BigInt.asUintN(64, arg0);
    return addHeapObject(ret);
}
export function __wbindgen_object_clone_ref(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
}
export function __wbindgen_object_drop_ref(arg0) {
    takeObject(arg0);
}
function __wasm_bindgen_func_elem_8936(arg0, arg1) {
    wasm.__wasm_bindgen_func_elem_8936(arg0, arg1);
}

function __wasm_bindgen_func_elem_2654(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_2654(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_2654_3(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_2654_3(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_8937(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_8937(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_2653(arg0, arg1) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_2653(retptr, arg0, arg1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wasm_bindgen_func_elem_11931(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_11931(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}


const __wbindgen_enum_ResizeObserverBoxOptions = ["border-box", "content-box", "device-pixel-content-box"];

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_export5(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getArrayU16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedInt16ArrayMemory0 = null;
function getInt16ArrayMemory0() {
    if (cachedInt16ArrayMemory0 === null || cachedInt16ArrayMemory0.byteLength === 0) {
        cachedInt16ArrayMemory0 = new Int16Array(wasm.memory.buffer);
    }
    return cachedInt16ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

let cachedInt8ArrayMemory0 = null;
function getInt8ArrayMemory0() {
    if (cachedInt8ArrayMemory0 === null || cachedInt8ArrayMemory0.byteLength === 0) {
        cachedInt8ArrayMemory0 = new Int8Array(wasm.memory.buffer);
    }
    return cachedInt8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint16ArrayMemory0 = null;
function getUint16ArrayMemory0() {
    if (cachedUint16ArrayMemory0 === null || cachedUint16ArrayMemory0.byteLength === 0) {
        cachedUint16ArrayMemory0 = new Uint16Array(wasm.memory.buffer);
    }
    return cachedUint16ArrayMemory0;
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export3(addHeapObject(e));
    }
}

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_export5(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_export5(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;


let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}
