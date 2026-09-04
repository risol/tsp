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

#ifndef __OBJC__
#import <Foundation/Foundation.h>
#elif USE(CF)
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
