/*
 * Copyright (C) 2026 Fady Farag.
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Library General Public
 * License as published by the Free Software Foundation; either
 * version 2 of the License, or (at your option) any later version.
 */

#pragma once

#include <concepts>
#include <wtf/Forward.h>
#include <wtf/Platform.h>

#ifdef __OBJC__
#import <Foundation/Foundation.h>
#else
// The standalone bridge is compiled as C++, while the WebKit header also
// supports Objective-C++. Keep an opaque C++ identity type so the trait is
// false for ordinary C++ and Core Foundation pointers without importing
// Objective-C declarations into a C++ translation unit.
struct TspObjectiveCObject;
using id = TspObjectiveCObject*;
#endif

#if !defined(__OBJC__) && USE(CF)
#include <CoreFoundation/CoreFoundation.h>
#endif

namespace WTF {

template<typename T>
inline constexpr bool IsNSType = std::convertible_to<T, id>;

template<typename T>
concept NSType = IsNSType<T>;

} // namespace WTF

using WTF::IsNSType;
using WTF::NSType;
