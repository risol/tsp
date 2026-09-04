/*
 * Compatibility subset for standalone JavaScriptCore embedders.
 *
 * The distributed Windows WebKit SDK used by TSP omits this public WTF
 * platform header even though PlatformEnable.h includes it for Windows.
 */
#pragma once

#ifndef WTF_PLATFORM_GUARD_AGAINST_INDIRECT_INCLUSION
#error "Please include <wtf/Platform.h> instead of this file directly."
#endif

#if !PLATFORM(WIN)
#error "This file should only be included when building the Windows port."
#endif

#if !defined(ENABLE_GEOLOCATION)
#define ENABLE_GEOLOCATION 1
#endif

#if !defined(ENABLE_OPENTYPE_MATH)
#define ENABLE_OPENTYPE_MATH 1
#endif

#if !defined(ENABLE_WEB_ARCHIVE)
#define ENABLE_WEB_ARCHIVE 1
#endif

#if !defined(ENABLE_WEBGL)
#define ENABLE_WEBGL 1
#endif

#if !defined(ENABLE_WEBPROCESS_CACHE)
#define ENABLE_WEBPROCESS_CACHE 1
#endif
