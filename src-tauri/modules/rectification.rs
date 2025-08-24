use opencv::{
    core::{Mat, Size},
    imgproc,
    prelude::*,
};
use std::time::Instant; // 添加性能监控

use crate::modules::param_io::*;

pub struct Rectifier {
    image_size: Size,
}

impl Rectifier {
    /// 创建校正器实例
    pub fn new(image_size: Size) -> Result<Self, opencv::Error> {
        Ok(Self {
            image_size,
        })
    }

    /// 3.3.1 应用重映射 - 🚀 优化版本
    pub fn remap_image(
        &self,
        src: &Mat,
        map1: &Mat,
        map2: &Mat,
    ) -> Result<Mat, opencv::Error> {
        let remap_start = Instant::now();
        let mut dst = Mat::default();
        
        // 🚀 使用优化的插值算法 - INTER_LINEAR替代INTER_CUBIC，速度更快
        imgproc::remap(
            src,
            &mut dst,
            map1,
            map2,
            imgproc::INTER_LINEAR,  // 保持线性插值，平衡速度和质量
            opencv::core::BORDER_CONSTANT,
            opencv::core::Scalar::default(),
        )?;

        let remap_time = remap_start.elapsed();
        println!("🔧 单次重映射耗时: {:.1} ms", remap_time.as_millis());

        Ok(dst)
    }
    
    /// 🚀 高性能重映射 - 针对实时处理优化
    pub fn remap_image_fast(
        &self,
        src: &Mat,
        map1: &Mat,
        map2: &Mat,
    ) -> Result<Mat, opencv::Error> {
        let remap_start = Instant::now();
        let mut dst = Mat::default();
        
        // 🚀 使用最快的插值算法 - INTER_NEAREST，牺牲少量质量换取速度
        imgproc::remap(
            src,
            &mut dst,
            map1,
            map2,
            imgproc::INTER_NEAREST,  // 最快的插值方法
            opencv::core::BORDER_CONSTANT,
            opencv::core::Scalar::default(),
        )?;

        let remap_time = remap_start.elapsed();
        println!("🚀 快速重映射耗时: {:.1} ms", remap_time.as_millis());

        Ok(dst)
    }
    
    /// 🔧 智能重映射 - 根据图像大小自动选择最优插值方法
    pub fn remap_image_adaptive(
        &self,
        src: &Mat,
        map1: &Mat,
        map2: &Mat,
    ) -> Result<Mat, opencv::Error> {
        let remap_start = Instant::now();
        let mut dst = Mat::default();
        
        // 根据图像大小选择插值方法
        let total_pixels = (src.cols() * src.rows()) as u64;
        let interpolation = if total_pixels > 4_000_000 {  // 大于4MP使用最快方法
            println!("🔧 大图像({:.1}MP)使用INTER_NEAREST", total_pixels as f64 / 1_000_000.0);
            imgproc::INTER_NEAREST
        } else {  // 小图像使用线性插值
            println!("🔧 中等图像({:.1}MP)使用INTER_LINEAR", total_pixels as f64 / 1_000_000.0);
            imgproc::INTER_LINEAR
        };
        
        imgproc::remap(
            src,
            &mut dst,
            map1,
            map2,
            interpolation,
            opencv::core::BORDER_CONSTANT,
            opencv::core::Scalar::default(),
        )?;

        let remap_time = remap_start.elapsed();
        println!("🔧 自适应重映射耗时: {:.1} ms", remap_time.as_millis());

        Ok(dst)
    }
}

// 可能需要的数据结构定义
pub struct RectifiedImage {
    pub image: Mat,
}
