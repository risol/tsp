/*
 * Compatibility subset for standalone JavaScriptCore embedders.
 *
 * Some distributed WebKit SDKs omit this public WTF header even though
 * RetainRef.h includes it. Keep the declarations in the SDK's namespace and
 * use the same trait names as WebKit so no engine-specific code is required.
 */
#pragma once

#include <wtf/Platform.h>

#if USE(CF)
#include <CoreFoundation/CoreFoundation.h>
#include <concepts>
#include <type_traits>

namespace WTF {

template <typename> struct CFTypeTrait;

} // namespace WTF

#define WTF_DECLARE_CF_TYPE_TRAIT(ClassName) \
    template <> \
    struct WTF::CFTypeTrait<ClassName##Ref> { \
        static inline CFTypeID typeID() { return ClassName##GetTypeID(); } \
    };

#define WTF_DECLARE_CF_MUTABLE_TYPE_TRAIT(ClassName, MutableClassName) \
    template <> \
    struct WTF::CFTypeTrait<MutableClassName##Ref> { \
        static inline CFTypeID typeID() { return ClassName##GetTypeID(); } \
    };

WTF_DECLARE_CF_TYPE_TRAIT(CFArray);
WTF_DECLARE_CF_TYPE_TRAIT(CFBoolean);
WTF_DECLARE_CF_TYPE_TRAIT(CFData);
WTF_DECLARE_CF_TYPE_TRAIT(CFDictionary);
WTF_DECLARE_CF_TYPE_TRAIT(CFError);
WTF_DECLARE_CF_TYPE_TRAIT(CFNumber);
WTF_DECLARE_CF_TYPE_TRAIT(CFRunLoop);
WTF_DECLARE_CF_TYPE_TRAIT(CFRunLoopSource);
WTF_DECLARE_CF_TYPE_TRAIT(CFRunLoopTimer);
WTF_DECLARE_CF_TYPE_TRAIT(CFString);
WTF_DECLARE_CF_TYPE_TRAIT(CFURL);
WTF_DECLARE_CF_MUTABLE_TYPE_TRAIT(CFArray, CFMutableArray);
WTF_DECLARE_CF_MUTABLE_TYPE_TRAIT(CFData, CFMutableData);
WTF_DECLARE_CF_MUTABLE_TYPE_TRAIT(CFDictionary, CFMutableDictionary);
WTF_DECLARE_CF_MUTABLE_TYPE_TRAIT(CFString, CFMutableString);

namespace WTF {

namespace detail {

template <typename T, typename = void>
inline constexpr bool HasCFTypeTraitHelper = false;

template <typename T>
inline constexpr bool HasCFTypeTraitHelper<
    T,
    std::void_t<decltype(CFTypeTrait<T>::typeID())>> = true;

} // namespace detail

template <typename T>
inline constexpr bool HasCFTypeTrait = detail::HasCFTypeTraitHelper<T>;

template <typename T>
inline constexpr bool IsCFType = std::is_pointer_v<T>
    && (std::same_as<std::remove_cv_t<T>, CFTypeRef> || HasCFTypeTrait<T>);

template <typename T>
concept CFType = IsCFType<T>;

} // namespace WTF

using WTF::CFType;
using WTF::HasCFTypeTrait;
using WTF::IsCFType;
#endif // USE(CF)
