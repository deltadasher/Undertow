cmake_minimum_required(VERSION 3.21)
project(undertow VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network WebEngineCore WebEngineWidgets)
find_package(LayerShellQt REQUIRED)
include(GNUInstallDirs)

add_executable(undertow
    src/main.cpp
    src/daemon.cpp
    src/library.cpp
    src/sysstats.cpp
)
target_link_libraries(undertow PRIVATE
    Qt6::Widgets
    Qt6::Network
    Qt6::WebEngineWidgets
    LayerShellQt::Interface
)

install(TARGETS undertow)
install(DIRECTORY presets/ DESTINATION ${CMAKE_INSTALL_DATADIR}/undertow/wallpapers)
